package dev.horizonlab.fabric.loader;

import java.io.File;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.Collection;
import java.util.Collections;
import java.util.EnumSet;
import java.util.List;
import java.util.Set;

import net.fabricmc.loader.impl.game.GameProvider;
import net.fabricmc.loader.impl.game.patch.GameTransformer;
import net.fabricmc.loader.impl.launch.FabricLauncher;
import net.fabricmc.loader.impl.metadata.BuiltinModMetadata;
import net.fabricmc.loader.impl.util.Arguments;
import net.fabricmc.loader.impl.util.log.Log;

/**
 * Presents Horizon to Fabric Loader as the game. Its "game jar" is the facade of Minecraft's classes
 * that forwards to Rust. It is never launched: Horizon is already running, see {@link Bootstrap}.
 */
public final class HorizonGameProvider implements GameProvider {
	public static final String GAME_JAR_PROPERTY = "horizon.fabric.gameJar";
	/** The libraries the real game ships (slf4j, Guava, LWJGL...), which mods use as their own. */
	public static final String GAME_LIBRARIES_PROPERTY = "horizon.fabric.gameLibraries";
	private static final String VERSION = "26.3";

	private static final Set<BuiltinTransform> GAME_TRANSFORMS = EnumSet.of(BuiltinTransform.WIDEN_ALL_PACKAGE_ACCESS, BuiltinTransform.CLASS_TWEAKS);
	private static final Set<BuiltinTransform> MOD_TRANSFORMS = EnumSet.of(BuiltinTransform.STRIP_ENVIRONMENT);

	private final GameTransformer transformer = new GameTransformer();
	private final List<Path> gameJars = new ArrayList<>(1);
	private Arguments arguments;

	@Override
	public String getGameId() {
		return "minecraft"; // what mods name in their `depends`
	}

	@Override
	public String getGameName() {
		return "Horizon";
	}

	@Override
	public String getRawGameVersion() {
		return VERSION;
	}

	@Override
	public String getNormalizedGameVersion() {
		return VERSION;
	}

	@Override
	public Collection<BuiltinMod> getBuiltinMods() {
		BuiltinModMetadata.Builder metadata = new BuiltinModMetadata.Builder(getGameId(), getNormalizedGameVersion()).setName(getGameName());
		return Collections.singletonList(new BuiltinMod(gameJars, metadata.build()));
	}

	@Override
	public String getEntrypoint() {
		return "dev.horizonlab.fabric.game.HorizonGame";
	}

	@Override
	public Path getLaunchDirectory() {
		return Paths.get(arguments == null ? "." : arguments.getOrDefault("gameDir", "."));
	}

	@Override
	public boolean requiresUrlClassLoader() {
		return false;
	}

	@Override
	public Set<BuiltinTransform> getBuiltinTransforms(String className) {
		boolean game = className.startsWith("net.minecraft.") || className.startsWith("com.mojang.");
		return game ? GAME_TRANSFORMS : MOD_TRANSFORMS;
	}

	@Override
	public boolean isEnabled() {
		return System.getProperty(GAME_JAR_PROPERTY) != null;
	}

	@Override
	public boolean locateGame(FabricLauncher launcher, String[] args) {
		arguments = new Arguments();
		arguments.parse(args);

		Path jar = Paths.get(System.getProperty(GAME_JAR_PROPERTY)).toAbsolutePath().normalize();
		if (!Files.isRegularFile(jar)) return false;

		gameJars.add(jar);
		Log.configureBuiltin(false, true);
		return true;
	}

	@Override
	public void initialize(FabricLauncher launcher) {
		launcher.setValidParentClassPath(launcher.getClassPath());
		transformer.locateEntrypoints(launcher, gameJars);
	}

	@Override
	public GameTransformer getEntrypointTransformer() {
		return transformer;
	}

	@Override
	public void unlockClassPath(FabricLauncher launcher) {
		for (Path jar : gameJars) launcher.addToClassPath(jar);

		String libraries = System.getProperty(GAME_LIBRARIES_PROPERTY, "");

		for (String library : libraries.split(File.pathSeparator)) {
			if (!library.isEmpty()) launcher.addToClassPath(Paths.get(library));
		}
	}

	@Override
	public void launch(ClassLoader loader) {
		throw new UnsupportedOperationException("Horizon is already running; Bootstrap starts the mods");
	}

	@Override
	public Arguments getArguments() {
		return arguments;
	}

	@Override
	public String[] getLaunchArguments(boolean sanitize) {
		return arguments == null ? new String[0] : arguments.toArray();
	}

	@Override
	public boolean canOpenErrorGui() {
		return false; // errors go back to Rust, which reports them in the game
	}

	@Override
	public boolean hasAwtSupport() {
		return false;
	}

	@Override
	public String getRuntimeNamespace(String defaultNs) {
		return "official"; // 26.x is unobfuscated: mods and the game share Mojang's names
	}

	@Override
	public String getDefaultModDistributionNamespace(String defaultNs) {
		return "official";
	}
}
