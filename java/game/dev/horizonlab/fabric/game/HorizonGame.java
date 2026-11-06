package dev.horizonlab.fabric.game;

import net.fabricmc.api.ClientModInitializer;
import net.fabricmc.api.ModInitializer;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.fabricmc.loader.impl.FabricLoaderImpl;
import net.minecraft.client.Minecraft;
import net.minecraft.client.player.LocalPlayer;

/** The game as Java sees it: Rust drives these, and the mods reach Rust through the native methods. */
public final class HorizonGame {
	private static final LocalPlayer PLAYER = new LocalPlayer();

	private HorizonGame() {
	}

	/** A line for the player's chat, as Rust's host collects it. */
	public static native void nativeSystemMessage(String text);

	public static void start() {
		FabricLoaderImpl.INSTANCE.invokeEntrypoints("main", ModInitializer.class, ModInitializer::onInitialize);
		FabricLoaderImpl.INSTANCE.invokeEntrypoints("client", ClientModInitializer.class, ClientModInitializer::onInitializeClient);
	}

	public static void clientTick(boolean inWorld) {
		Thread.currentThread().setContextClassLoader(HorizonGame.class.getClassLoader());

		Minecraft client = Minecraft.getInstance();
		client.player = inWorld ? PLAYER : null;
		ClientTickEvents.START_CLIENT_TICK.invoker().onStartTick(client);
		ClientTickEvents.END_CLIENT_TICK.invoker().onEndTick(client);
	}
}
