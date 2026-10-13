# SoundCraft AI Studio 0.1.1 — natives Panel

Stand: 2026-10-13. Native Entwicklungsstufe mit lokal geprüfter Oberfläche und erfolgreichen Plattform-Builds/-Tests.

## Bedienung

`Window > AI Studio` öffnet oder schließt das verschiebbare und skalierbare native Fenster. Seine Sichtbarkeit wird mit den bestehenden UI-Einstellungen gespeichert. Alte Einstellungen funktionieren weiterhin und starten mit geschlossenem Panel.

Das Panel zeigt das aktuelle Projekt, Spuren und Clip-Anzahlen. Es aktualisiert die Übersicht bei Dokumentänderungen; `Aktualisieren` liest den Zustand erneut. Die Spurliste scrollt separat, damit Systemprüfung und Diagnose erreichbar bleiben. Ein Projektwechsel ersetzt den Kontext; das Schließen des Panels entfernt die gespeicherte Projektübersicht aus dem Panelzustand.

`System prüfen` verwendet die isolierte Prüfung aus Phase 1 auf einem eigenen Hintergrundthread. Mehrfachstarts sind gesperrt. Die laufende Session wird weder geöffnet, gespeichert noch bearbeitet. Ergebnisse erscheinen im Panel.

`Diagnose kopieren` gibt den JSON-Bericht in die Zwischenablage. Die Desktop-App bietet außerdem `Diagnose speichern` mit einem JSON-Dateidialog. Der Bericht enthält technische Metadaten und den isolierten Selbsttest, keine Projektnamen, Parameter, freien Backend-Fehlertexte, Zugangsdaten oder musikalischen Inhalte.

**Noch keine KI-Anbieter oder Chatfunktion.** Das Fenster zeigt diesen Entwicklungsstand ausdrücklich an. Provider-Anbindung, Schlüsselablage und musikalische Bearbeitungen folgen in Phase 3.

## Integration

Ein neues UI-Modul `crates/ui-egui/src/ai_studio.rs`, ein UI-Zustandsflag, ein Fensterbefehl und ein Aufruf im bestehenden UI-Renderpfad. Die UI verwendet die AI Bridge mit einem geliehenen, ausschließlich lesenden Adapter auf die vorhandene Befehlschnittstelle. Keine zweite Engine für die Projektübersicht, keine TCP-Schleife im UI-Thread und keine direkten Session-Schreibzugriffe. Der Hintergrundtest erzeugt seine eigene Engine.

Die Audio-Engine, DSP, Playback und Plugin-Hosts bleiben unverändert. Die Desktop-Integration ergänzt lediglich den JSON-Dateifilter. Rust 1.95 ist für die bestehende UI erforderlich. Die SoundCraft-Basis bleibt 0.6.1; die Erweiterung besitzt die eigene Version 0.1.1.

## Tatsächlich lokal geprüft

Linux x86_64, Rust 1.95. Echte Engine und echte egui-Oberfläche.

- 3 neue UI-Tests: Fensterbefehl/gespeicherte Sichtbarkeit/Legacy-Einstellungen, reales Rendern mit unverändertem Projekt und ohne musikalische Diagnosedaten, Projektwechsel/Schließen sowie asynchroner Selbsttest.
- 4 bestehende UI-Tests: Befehlsfilter und Erreichbarkeit aus Menüs.
- 25 Tests von AI Studio und Automation erneut bestanden.
- Clippy für AI Studio und UI einschließlich aller Targets, Formatierung, Architekturprüfung und Asset-Prüfung bestanden.
- Die echte Oberfläche mit `ui_shot` gerendert und bildlich geprüft. Diagnosebedienung sichtbar; Spurliste separat scrollbar.

Die lokale Werkzeugkette benötigte zusätzliche Linux-Entwicklungspakete. Sie wurden in einen separaten Arbeitsordner extrahiert; Buildprofile und Produktabhängigkeiten wurden dafür nicht verändert. Die lokalen UI-Builds nutzten geringe Parallelität, deaktivierte Optimierung und einen Codegen-Block, um Fehler beim Erzeugen von Abhängigkeitsarchiven in dieser Umgebung zu vermeiden.

## Native Builds und Grenzen

Der CI-Workflow baut die Desktop-App und führt die gezielten Modul- und UI-Tests unter macOS, Windows und Linux aus. Linux erzeugt außerdem einen gerenderten Screenshot. Die Artefakte sind unsignierte Entwicklungsprogramme und Diagnosen, keine freigegebenen Installationspakete. Der vollständige Workflow wurde für Commit `dc6b4e4b2136511db59165dab86d572a95e5bc22` tatsächlich ausgeführt und geprüft:

| Plattform | Tests und Desktop-Build |
|---|---|
| Windows | Erfolgreich |
| macOS | Erfolgreich |
| Linux | Erfolgreich, einschließlich Screenshot |

[Nachweis und Entwicklungsartefakte: GitHub Actions, Lauf 38459644221](https://github.com/wibem1/soundcraft/actions/runs/38459644221).

Die Tests der Bridge, Automation und UI, die Clippy-Prüfung sowie Architektur-/Asset-Prüfung und der isolierte Selbsttest bestanden auf allen drei Plattformen. Zusätzlich wurde die Desktop-App gebaut. Diese anschließende Dokumentationsänderung verändert keinen Code.

Ein erfolgreicher Build ersetzt keinen praktischen Audio-/Plugin-Test und keine vollständige Desktop-Abnahme.

Der vollständige Workspace-/WASM-Release-Gate bleibt offen. Das native Panel wurde lokal geprüft; die in Phase 1 dokumentierten Grenzen werden nicht als bestanden umgedeutet.
