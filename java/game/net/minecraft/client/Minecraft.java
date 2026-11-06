package net.minecraft.client;

import net.minecraft.client.player.LocalPlayer;

public class Minecraft {
	private static final Minecraft INSTANCE = new Minecraft();

	public LocalPlayer player;

	private Minecraft() {
	}

	public static Minecraft getInstance() {
		return INSTANCE;
	}
}
