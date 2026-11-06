package dev.horizonlab.fabric.testmod;

import net.fabricmc.api.ClientModInitializer;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.fabricmc.loader.api.FabricLoader;
import net.minecraft.client.Minecraft;
import net.minecraft.network.chat.Component;

/** Says hello once, on the first tick in a world. Written as any Fabric mod for 26.3 would be. */
public final class HelloMod implements ClientModInitializer {
	private boolean greeted;

	@Override
	public void onInitializeClient() {
		String version = FabricLoader.getInstance().getModContainer("horizon_fabric_testmod").orElseThrow().getMetadata().getVersion().getFriendlyString();

		ClientTickEvents.END_CLIENT_TICK.register((Minecraft client) -> {
			if (greeted || client.player == null) return;
			greeted = true;
			client.player.sendSystemMessage(Component.literal("Hello from a Fabric mod " + version));
		});
	}
}
