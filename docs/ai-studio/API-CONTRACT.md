# AI Bridge — offizieller Schnittstellenvertrag

## Offizielle Quellen
- [Control Protocol](../control-protocol.md) — localhost JSON-Lines, engine.execute, engine.commands, session.inspect, ui.inspect.
- [MCP](../mcp.md) — list_commands, execute, batch, inspect_session und Projektdienste.
- Laufzeitreferenz: `soundcraft-cli commands`, `soundcraft-cli describe <id>`.
- Integrationstest-Vorbild: `crates/automation/tests/agent_tasks.rs`.

## Vertragsprüfung pro Aufruf
Befehls-ID, Parameter, Vorbedingungen, Antwort, Fehler, verifizierbare Nachbedingung, Undo-Verhalten und Version dokumentieren. Menschenlesbare Parametertexte sind keine vollständigen maschinenlesbaren Schemata.

## Erste Testabdeckung
| Funktion | Befehl / Schnittstelle |
|---|---|
| Kommandoliste | engine.commands |
| Projektzustand | session.inspect |
| Spur erzeugen | track.new |
| MIDI-Clip erzeugen | midi.new_clip |
| Note hinzufügen / ändern | midi.note_add / midi.note_edit |
| Mixerpegel | mix.volume |
| Rückgängig | edit.undo |
| Projekt laden / speichern | session.open / session.save |

## Update-Gates
1. Offizielle Releases beobachten; keine automatische Übernahme.
2. API-Snapshot und MCP-Tools der Referenz- und Kandidatenversion vergleichen.
3. Verhaltensprüfung in isoliertem Testprojekt: MIDI, Mixer, Undo, Save/Load.
4. Native Erweiterung separat für macOS und Windows bauen und testen.
5. Status: KOMPATIBEL / INKOMPATIBEL / NICHT VOLLSTÄNDIG GEPRÜFT.
6. Nur nach ausdrücklicher Freigabe aktualisieren. Bewährte Version erhalten.

## Grenzen
API-Discovery beweist nicht unverändertes Verhalten. Ein grüner Test bedeutet Kompatibilität nur für die abgedeckten Workflows. Headless-Root-Restriktionen gelten nicht automatisch für die angeschlossene Desktop-App; die Bridge muss Schreibaktionen gesondert autorisieren.
