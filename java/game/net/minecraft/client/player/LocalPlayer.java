package net.minecraft.client.player;

import dev.horizonlab.fabric.game.HorizonGame;
import net.minecraft.network.chat.Component;

public class LocalPlayer {
	public void sendSystemMessage(Component message) {
		HorizonGame.nativeSystemMessage(message.getString());
	}
}
