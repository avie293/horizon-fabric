//! Boots the host without the game and ticks it: `build.py --smoke` runs this.
//!
//! fabric_smoke <java home> <dist folder> <game dir> [expected message]

use horizon_fabric_host::{Config, FabricHost, Outgoing};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn jars(folder: &Path) -> Vec<PathBuf> {
    let mut jars: Vec<PathBuf> = std::fs::read_dir(folder)
        .map(|entries| entries.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    jars.retain(|p| p.extension().is_some_and(|e| e == "jar"));
    jars.sort();
    jars
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [java_home, dist, game_dir, rest @ ..] = args.as_slice() else {
        eprintln!("usage: fabric_smoke <java home> <dist folder> <game dir> [expected message]");
        return ExitCode::from(2);
    };
    let dist = Path::new(dist);
    let mut classpath = jars(&dist.join("libs"));
    classpath.push(dist.join("horizon-fabric-provider.jar"));
    let config = Config {
        java_home: java_home.into(),
        classpath,
        game_jar: dist.join("horizon-fabric-game.jar"),
        game_libraries: jars(&dist.join("game-libs")),
        game_dir: game_dir.into(),
    };

    let host = match FabricHost::start(&config) {
        Ok(host) => host,
        Err(e) => {
            eprintln!("FAILED to start:\n{e}");
            return ExitCode::FAILURE;
        }
    };
    println!("host started");

    // Two ticks on the title screen and two in a world: the mod must speak once, on entering it.
    let mut messages = Vec::new();
    for in_world in [false, false, true, true] {
        match host.client_tick(in_world) {
            Ok(out) => messages.push((in_world, out)),
            Err(e) => {
                eprintln!("FAILED in a tick:\n{e}");
                return ExitCode::FAILURE;
            }
        }
    }
    println!("per tick: {messages:?}");

    if let Some(expected) = rest.first() {
        let want = vec![
            (false, vec![]),
            (false, vec![]),
            (true, vec![Outgoing::SystemMessage(expected.clone())]),
            (true, vec![]),
        ];
        if messages != want {
            eprintln!("FAILED: wanted {want:?}");
            return ExitCode::FAILURE;
        }
        println!("PASS");
    }
    ExitCode::SUCCESS
}
