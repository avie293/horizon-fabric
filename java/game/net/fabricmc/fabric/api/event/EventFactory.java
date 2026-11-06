package net.fabricmc.fabric.api.event;

import java.lang.reflect.Array;
import java.util.function.Function;

public final class EventFactory {
	private EventFactory() {
	}

	public static <T> Event<T> createArrayBacked(Class<? super T> type, Function<T[], T> invokerFactory) {
		return new ArrayBacked<>(type, invokerFactory);
	}

	private static final class ArrayBacked<T> extends Event<T> {
		private final Function<T[], T> invokerFactory;
		private T[] listeners;

		@SuppressWarnings("unchecked")
		ArrayBacked(Class<? super T> type, Function<T[], T> invokerFactory) {
			this.invokerFactory = invokerFactory;
			this.listeners = (T[]) Array.newInstance(type, 0);
			this.invoker = invokerFactory.apply(listeners);
		}

		@Override
		public synchronized void register(T listener) {
			T[] grown = java.util.Arrays.copyOf(listeners, listeners.length + 1);
			grown[listeners.length] = listener;
			listeners = grown;
			invoker = invokerFactory.apply(grown);
		}
	}
}
