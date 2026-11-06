# horizon-fabric

Hobbyprojekt: Fabric-Mods in [Horizon](https://github.com/horizon-rust/horizon) (Minecraft 26.3 in Rust) laufen lassen.
Soll sich später als extra Installation in eine einzelne Instanz einhaken, Horizon selbst bleibt unverändert.

Ob das jemals richtig läuft weiß ich nicht, ist ziemlich unrealistisch und viel Arbeit. Aber macht Spaß.

## Wie das funktioniert

```
Rust (host/)                 startet eine JVM im selben Prozess und tickt die Mods
 └─ Fabric Loader 0.20.1     unverändert, von Fabrics Maven
     ├─ HorizonGameProvider  java/provider/ - sagt dem Loader dass Horizon das Spiel ist
     ├─ Fassade              java/game/ - Minecraft- und Fabric-API-Klassen, leiten nach Rust weiter
     └─ Mods                 <game dir>/mods/
```

Die Fassade muss exakt zum echten Spiel passen, weil Mods über Name und Signatur linken.
Mit `--vanilla-jar` wird das gegen den echten Client geprüft.

## Bauen und testen

Braucht ein JDK ab Version 25 und Cargo. Alles landet in `.local/`.

```sh
python build.py --java-home <JDK> --smoke        # der Testmod muss über seinen Mixin Hallo sagen
python build.py --java-home <JDK> --mods a.jar   # echten Mod ausprobieren und schauen wie er scheitert
#   --vanilla-jar <26.3 client jar>    Fassade gegen das echte Spiel prüfen
#   --version-json <26.3.json>         Bibliotheken vom echten Spiel holen (slf4j, Guava, LWJGL...)
```

Spiel-Jars und Bibliotheken kommen von Mojang und haben hier im Repo nichts zu suchen.

## Stand

Läuft: `--smoke`. Die JVM startet aus Rust, der Loader findet den Testmod, der Mixin patcht `LocalPlayer`
und die Chat-Nachricht kommt in Rust an. Das alles aber noch ohne das echte Spiel.

Probleme:

- Mods bringen ihre eigenen Fabric-API-Module mit, und die überdecken die Klassen aus der Fassade.
  Deren `Event` braucht dann zum Beispiel `net.minecraft.resources.Identifier`.
- Ein Mixin braucht das was er patcht. `@Shadow` und Injection Points zeigen auf Sachen mitten in Vanilla-Methoden,
  eine leere Klasse reicht also nicht.
- Sodium und Iris ersetzen einen Renderer den Horizon gar nicht hat. Selbst wenn die durchladen, zeichnen sie nichts,
  weil die Bilder aus wgpu kommen.

## Als nächstes

Den Host als Native Mod ins echte Spiel einbauen (Tick-Hook, Chat).
