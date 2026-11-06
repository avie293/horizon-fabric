package net.minecraft.network.chat;

public interface Component {
	default String getString() {
		return ((MutableComponent) this).text();
	}

	static MutableComponent literal(String text) {
		return new MutableComponent(text);
	}
}
