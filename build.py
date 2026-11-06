#!/usr/bin/env python3
"""Builds horizon-fabric's Java side into .local/ (ignored by git) and tests it.

  python build.py --java-home <JDK 25+>
      fetch Fabric Loader's libraries, compile the provider, the facade and the test mod
  ... --smoke
      boot the host without the game in a fresh folder, with the test mod; must say hello
  ... --mods a.jar b.jar
      boot the host with these mods instead, and report what happens
  ... --vanilla-jar <26.3 client jar>
      check that every public member of the facade exists in the real game, same signature
  ... --version-json <26.3.json>
      fetch the libraries the real game ships, for the mods that use them
  ... --install <instance folder>
      copy the runtime to <instance folder>/horizon-fabric/
"""
import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
import urllib.request
import zipfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
FABRIC_MAVEN = "https://maven.fabricmc.net/"
# Fabric's own profile for 26.3 and loader 0.20.1: meta.fabricmc.net/v2/versions/loader/26.3/0.20.1/profile/json
LIBRARIES = [
    "org.ow2.asm:asm:9.10.1",
    "org.ow2.asm:asm-analysis:9.10.1",
    "org.ow2.asm:asm-commons:9.10.1",
    "org.ow2.asm:asm-tree:9.10.1",
    "org.ow2.asm:asm-util:9.10.1",
    "net.fabricmc:sponge-mixin:0.17.5+mixin.0.8.7",
    "net.fabricmc:fabric-loader:0.20.1",
]
EXPECTED_HELLO = "[mixin] Hello from a Fabric mod 1.2.3"


def fetch(url):
    request = urllib.request.Request(url, headers={"User-Agent": "horizon-fabric-build"})
    with urllib.request.urlopen(request, timeout=60) as response:
        return response.read()


def fetch_libraries(libs):
    libs.mkdir(parents=True, exist_ok=True)
    for coordinate in LIBRARIES:
        group, artifact, version = coordinate.split(":")
        name = f"{artifact}-{version}.jar"
        target = libs / name
        if target.exists():
            continue
        url = f"{FABRIC_MAVEN}{group.replace('.', '/')}/{artifact}/{version}/{name}"
        data = fetch(url)
        want = fetch(url + ".sha1").decode().strip().split()[0]
        got = hashlib.sha1(data).hexdigest()
        if got != want:
            sys.exit(f"{name}: sha1 {got}, the repository says {want}")
        target.write_bytes(data)
        print(f"fetched {name}")


def rules_allow(rules):
    """A version JSON's rules for this system: the last rule that applies decides."""
    system = {"nt": "windows", "posix": "osx" if sys.platform == "darwin" else "linux"}[os.name]
    allowed = not rules
    for rule in rules or []:
        if rule.get("os", {}).get("name", system) == system:
            allowed = rule["action"] == "allow"
    return allowed


def fetch_game_libraries(version_json, folder):
    """The libraries the real game ships, from Mojang's links in its version JSON. Never committed."""
    folder.mkdir(parents=True, exist_ok=True)
    for library in json.loads(version_json.read_text(encoding="utf-8"))["libraries"]:
        artifact = library.get("downloads", {}).get("artifact")
        if not artifact or not rules_allow(library.get("rules")):
            continue
        target = folder / Path(artifact["path"]).name
        if target.exists():
            continue
        data = fetch(artifact["url"])
        got = hashlib.sha1(data).hexdigest()
        if got != artifact["sha1"]:
            sys.exit(f"{target.name}: sha1 {got}, the version JSON says {artifact['sha1']}")
        target.write_bytes(data)
    print(f"game libraries: {len(list(folder.glob('*.jar')))}")


def tool(java_home, name):
    suffix = ".exe" if os.name == "nt" else ""
    return str(Path(java_home) / "bin" / (name + suffix))


def compile_jar(java_home, sources, classpath, out_jar, extra_files=None):
    """Compiles every .java under `sources` and zips the classes, with the other files named."""
    java_files = [p.as_posix() for p in sorted(sources.rglob("*.java"))]
    with tempfile.TemporaryDirectory() as tmp:
        classes = Path(tmp) / "classes"
        argfile = Path(tmp) / "sources.txt"
        argfile.write_text("\n".join(f'"{p}"' for p in java_files), encoding="utf-8")
        subprocess.run(
            [tool(java_home, "javac"), "--release", "21", "-proc:none", "-nowarn", "-d", str(classes),
             "-cp", os.pathsep.join(str(p) for p in classpath), f"@{argfile}"],
            check=True,
        )
        out_jar.parent.mkdir(parents=True, exist_ok=True)
        with zipfile.ZipFile(out_jar, "w", zipfile.ZIP_DEFLATED) as jar:
            for path in sorted(classes.rglob("*")):
                if path.is_file():
                    jar.write(path, path.relative_to(classes).as_posix())
            for path, name in extra_files or []:
                jar.write(path, name)
    print(f"built {out_jar.name}")


def resources(folder, root=None):
    """The files under `folder`, named as they are from `root` (the folder itself by default)."""
    root = root or folder
    return [(p, p.relative_to(root).as_posix()) for p in sorted(folder.rglob("*")) if p.is_file()]


def build(java_home, out):
    libs = out / "libs"
    fetch_libraries(libs)
    lib_jars = sorted(libs.glob("*.jar"))
    provider = HERE / "java" / "provider"
    compile_jar(java_home, provider, lib_jars, out / "horizon-fabric-provider.jar",
                resources(provider / "META-INF", provider))
    game = out / "horizon-fabric-game.jar"
    compile_jar(java_home, HERE / "java" / "game", lib_jars, game)
    compile_jar(java_home, HERE / "testmod" / "src", [*lib_jars, game], out / "testmod" / "horizon-fabric-testmod.jar",
                resources(HERE / "testmod" / "resources"))


def members(java_home, classpath, class_name):
    """The public members javap lists for a class, as `(signature)` lines; None if it doesn't exist."""
    run = subprocess.run([tool(java_home, "javap"), "-public", "-cp", str(classpath), class_name],
                         capture_output=True, text=True)
    if run.returncode != 0:
        return None
    found = set()
    for line in run.stdout.splitlines():
        line = line.strip()
        if not line.endswith(";") or class_name + "(" in line:  # constructors are the facade's own
            continue
        found.add(line.replace("default ", "").replace("abstract ", "").replace("final ", ""))
    return found


def check_facade(java_home, out, vanilla_jar):
    """Mods link by exact descriptor, so a member the real game lacks would fail only for real mods."""
    game = out / "horizon-fabric-game.jar"
    with zipfile.ZipFile(game) as jar:
        names = [n[:-6].replace("/", ".") for n in jar.namelist()
                 if n.endswith(".class") and n.startswith(("net/minecraft/", "com/mojang/")) and "$" not in n]
    wrong = 0
    for name in names:
        ours = members(java_home, game, name)
        real = members(java_home, vanilla_jar, name)
        if real is None:
            print(f"NOT IN THE GAME: {name}")
            wrong += 1
            continue
        for member in sorted(ours - real):
            print(f"NOT IN THE GAME: {name}: {member}")
            wrong += 1
    print(f"facade check: {len(names)} classes, {wrong} problems")
    return wrong == 0


def run_host(java_home, out, mods, expected):
    target = out / "target"
    env = {**os.environ, "CARGO_TARGET_DIR": str(target)}
    host = HERE / "bundle" / "host"
    subprocess.run(["cargo", "build", "--manifest-path", str(host / "Cargo.toml"), "--bin", "fabric_smoke"], check=True, env=env)
    (host / "Cargo.lock").unlink(missing_ok=True)  # a native mod's crate isn't allowed to have one
    binary = target / "debug" / ("fabric_smoke.exe" if os.name == "nt" else "fabric_smoke")
    with tempfile.TemporaryDirectory() as game_dir:
        (Path(game_dir) / "mods").mkdir()
        for mod in mods:
            shutil.copy(mod, Path(game_dir) / "mods")
        command = [str(binary), java_home, str(out), game_dir]
        if expected:
            command.append(expected)
        return subprocess.run(command, cwd=game_dir).returncode


def install(java_home, out, instance):
    """Copies the runtime to <instance>/horizon-fabric/. Leaves its mods/ and config/ alone."""
    target = instance / "horizon-fabric"
    (target / "mods").mkdir(parents=True, exist_ok=True)
    for folder in ("libs", "game-libs"):
        shutil.rmtree(target / folder, ignore_errors=True)
        if (out / folder).exists():
            shutil.copytree(out / folder, target / folder)
    for jar in ("horizon-fabric-provider.jar", "horizon-fabric-game.jar"):
        shutil.copy(out / jar, target / jar)
    (target / "java-home.txt").write_text(str(Path(java_home).resolve()), encoding="utf-8")
    print(f"installed to {target}")


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--java-home", default=os.environ.get("JAVA_HOME"), help="a JDK, 25 or later")
    parser.add_argument("--out", type=Path, default=HERE / ".local")
    parser.add_argument("--smoke", action="store_true")
    parser.add_argument("--mods", nargs="+", type=Path)
    parser.add_argument("--vanilla-jar", type=Path)
    parser.add_argument("--version-json", type=Path)
    parser.add_argument("--install", type=Path, metavar="INSTANCE")
    args = parser.parse_args()
    if not args.java_home:
        sys.exit("--java-home or JAVA_HOME: a JDK, 25 or later")

    build(args.java_home, args.out)
    if args.version_json:
        fetch_game_libraries(args.version_json, args.out / "game-libs")
    status = 0
    if args.vanilla_jar and not check_facade(args.java_home, args.out, args.vanilla_jar):
        status = 1
    if args.smoke:
        status |= run_host(args.java_home, args.out, [args.out / "testmod" / "horizon-fabric-testmod.jar"], EXPECTED_HELLO)
    if args.mods:
        status |= run_host(args.java_home, args.out, args.mods, None)
    if args.install:
        install(args.java_home, args.out, args.install)
    sys.exit(status)


main()
