// Keep the host alive from the game's frame, and show what the mods said in chat.
fn horizon_fabric_frame(game: &mut crate::game::Game<'_>) {
    let in_world = game.native_clock().is_some();
    let lines = horizon_fabric_host::on_frame(|| game.native_instance_dir(), in_world);
    for line in lines {
        game.native_chat(&line);
    }
}
