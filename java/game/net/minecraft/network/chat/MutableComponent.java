package net.minecraft.network.chat;

public final class MutableComponent implements Component {
	private final String text;

	MutableComponent(String text) {
		this.text = text;
	}

	String text() {
		return text;
	}
}
