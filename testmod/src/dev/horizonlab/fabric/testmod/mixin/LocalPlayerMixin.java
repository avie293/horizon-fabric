package dev.horizonlab.fabric.testmod.mixin;

import net.minecraft.client.player.LocalPlayer;
import net.minecraft.network.chat.Component;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.ModifyVariable;

/** Proves Mixin really patches a class: the message reaching Rust carries this prefix. */
@Mixin(LocalPlayer.class)
abstract class LocalPlayerMixin {
	@ModifyVariable(method = "sendSystemMessage", at = @At("HEAD"), argsOnly = true)
	private Component horizonFabricTestmod$prefix(Component message) {
		return Component.literal("[mixin] " + message.getString());
	}
}
