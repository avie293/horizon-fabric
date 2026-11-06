# horizon-fabric

Getting Fabric mods to run in [Horizon](https://github.com/horizon-rust/horizon) (Minecraft 26.3 in Rust).
It hooks into a single instance as an extra install, Horizon itself stays untouched.

No idea if this will ever really work. It's pretty unrealistic and a lot of work, but it's fun.

## How it works

```
Rust (bundle/host/)          starts a JVM in the game's process and ticks the mods
 └─ Fabric Loader 0.20.1     unchanged, from Fabric's Maven
     ├─ HorizonGameProvider  java/provider/ - tells the loader that Horizon is the game
     ├─ facade               java/game/ - Minecraft and Fabric API classes, forwarding to Rust
     └─ mods                 <instance>/horizon-fabric/mods/
```

The facade has to match the real game exactly, because mods link by name and signature.
`--vanilla-jar` checks it against the real client.

## Building and testing

Needs a JDK 25 or newer and Cargo. Everything ends up in `.local/`.

```sh
python build.py --java-home <JDK> --smoke        # the test mod has to say hello through its mixin
python build.py --java-home <JDK> --mods a.jar   # try a real mod and see how it fails
#   --vanilla-jar <26.3 client jar>    check the facade against the real game
#   --version-json <26.3.json>         fetch the libraries the real game ships (slf4j, Guava, LWJGL...)
#   --install <instance folder>        copy the runtime into an instance
```

Game jars and libraries come from Mojang and don't belong in this repo.

## Installing into an instance

1. `python build.py --java-home <JDK> --version-json <26.3.json> --install <instance folder>`
2. `horizon-launcher native-build bundle --instance <instance> --trust-native-code`
3. Put mods in `<instance folder>/horizon-fabric/mods/`

## Status

Works: `--smoke`. The JVM starts from Rust, the loader finds the test mod, its mixin patches `LocalPlayer`
and the chat message reaches Rust.

Problems:

- Mods bring their own Fabric API modules and those shadow the classes in the facade. Their `Event` needs
  `net.minecraft.resources.Identifier`, for example.
- A mixin needs the thing it patches. `@Shadow` and injection points point at stuff in the middle of vanilla
  methods, so an empty class isn't enough.
- Sodium and Iris replace a renderer Horizon doesn't have. Even if they load, they won't draw anything, because
  the frames come from wgpu.
