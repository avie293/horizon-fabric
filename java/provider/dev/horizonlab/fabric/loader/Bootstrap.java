package dev.horizonlab.fabric.loader;

import java.io.PrintWriter;
import java.io.StringWriter;

import net.fabricmc.api.EnvType;
import net.fabricmc.loader.impl.launch.knot.Knot;

/** What Rust calls in the system class loader. */
public final class Bootstrap {
	private Bootstrap() {
	}

	/**
	 * Runs Fabric Loader up to the point where the game would start: finds and resolves the mods, sets
	 * up Mixin. Knot.init and not Knot.launch, which ends the process when a mod fails to load.
	 *
	 * @return the game's class in Knot's class loader, not yet initialised, so Rust can register its
	 *         native methods first
	 */
	public static Class<?> boot(String gameDir) throws ClassNotFoundException {
		ClassLoader loader = new Knot(EnvType.CLIENT).init(new String[] {"--gameDir", gameDir});
		return Class.forName("dev.horizonlab.fabric.game.HorizonGame", false, loader);
	}

	public static String describe(Throwable error) {
		StringWriter out = new StringWriter();
		error.printStackTrace(new PrintWriter(out, true));
		return out.toString();
	}
}
