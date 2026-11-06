package net.fabricmc.fabric.api.event;

/** Fabric API's event, same shape: mods compiled against the real one link against this. */
public abstract class Event<T> {
	protected volatile T invoker;

	public final T invoker() {
		return invoker;
	}

	public abstract void register(T listener);
}
