//! Boots the host without the game and ticks it: `build.py --smoke` runs this.
//!
//! fabric_smoke <java home> <dist folder> <game dir> [expected message]

use horizon_fabric_host::{Config, FabricHost, Outgoing};
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [java_home, dist, game_dir, rest @ ..] = args.as_slice() else {
        eprintln!("usage: fabric_smoke <java home> <dist folder> <game dir> [expected message]");
        return ExitCode::from(2);
    };
    let config = Config::from_dist(java_home.into(), Path::new(dist), game_dir.into());

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
