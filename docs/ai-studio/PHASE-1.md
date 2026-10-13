# SoundCraft AI Studio 0.1.0 — Phase 1

Stand: 2026-10-13. Entwicklungsstufe, keine freigegebene Desktop-App.

## Implementiert

- Eigenständiges Rust-Modul `crates/ai-studio`, Layer 5; kein Eingriff in Audio-Engine, Playback, DSP oder Plugin-Hosts.
- AI Bridge über den vorhandenen `Backend`-Vertrag. Abfragen und eine begrenzte Befehlsliste; Schreibbefehle brauchen eine vom Host gesetzte Autorisierung. Modellantworten dürfen diese Autorisierung nicht selbst erteilen.
- Loopback-Transport für den dokumentierten JSON-Lines-Control-Kanal: Zeitlimits, Größenbegrenzung, Antwort-ID-Prüfung und keine automatische Wiederholung einer möglicherweise bereits ausgeführten Änderung.
- Compatibility Manager: versionierter API-Snapshot, erforderliche Befehle, entfernte/neue IDs und geänderte Parameterdokumentation. Wechselnde `enabled`-Zustände werden nicht als API-Änderung behandelt.
- Referenz: SoundCraft 0.6.1, Upstream-Commit `c0cb99340891d306e216582db465f3594b89dc1d`, 478 Engine-Befehle in `API-BASELINE-0.6.1.json`. Der Desktop ergänzt UI-Befehle; diese Headless-Referenz deckt sie nicht ab. Kein Update der Grundlage.
- Isolierter Selbsttest: neues synthetisches Projekt, neue temporäre Ordner, eingeschränkte Dateizugriffe, MIDI erzeugen/bearbeiten/Undo, Mixer und Save/Open einschließlich Nachbedingungen. Der Selbsttest verbindet sich niemals mit einem bestehenden Benutzerprojekt.
- JSON-Diagnose: Plattform, Architektur, Version, Build-Commit, feste Basisversion/-commit, Zeitpunkt, Dauer, Befehls-ID und Fehlerkategorie. Fehler des Workflows nennen den betroffenen Prüfschritt. Keine freien Fehlertexte, Parameter, Antworten, Pfade, Zugangsdaten oder musikalischen Inhalte. Maximal 4096 protokollierte Ereignisse. Noch keine KI-Aufrufe: Modell/Provider leer, Tokens und API-Kosten null bzw. 0.
- CI-Matrix für native Headless-Tests auf macOS, Windows und Linux. Der Workflow erzeugt Diagnoseartefakte; er veröffentlicht keine Installationspakete.

## Benutzung

```sh
cargo run -p soundcraft-ai-studio -- self-test
cargo run -p soundcraft-ai-studio -- headless-snapshot
# SoundCraft zuvor mit --control 0 starten; tatsächlichen Port/Version/Commit einsetzen:
cargo run -p soundcraft-ai-studio -- snapshot PORT VERSION FULL_COMMIT_SHA > candidate.json
cargo run -p soundcraft-ai-studio -- compare docs/ai-studio/API-BASELINE-0.6.1.json candidate.json
```

Exit-Codes: 0 = Selbsttest bestanden bzw. Snapshot erstellt, 1 = Aufruf-/Datenfehler, 2 = Selbsttest fehlgeschlagen oder Kompatibilität nicht vollständig geprüft/inkompatibel. Der API-Vergleich gibt bewusst auch bei unverändertem Vertrag `NotFullyChecked` aus. Discovery allein erteilt niemals eine Update-Freigabe. Version und Commit eines extern angeschlossenen SoundCraft müssen vom Host verifiziert werden; das Protokoll liefert hier keine authentifizierte Versionsabfrage.

## Testumfang und verbleibende Grenzen

Die Tests prüfen die echte Engine über die offizielle Headless-Automation sowie einen lokalen TCP-Testserver. Ein TCP-Testserver ist kein getesteter SoundCraft-Desktop. Das native KI-Panel, Provider-Anbindung, erweiterte musikalische Diagnose mit Opt-in, MCP-Tool-Snapshots, Kandidaten-Verhaltenstests gegen den Desktop und vollständige Release-Gates folgen in weiteren Schritten.

Dateiberechtigungen einer Desktop-Verbindung werden nicht durch die Headless-Roots garantiert: Der Host muss pro Aktion den zulässigen Projekt-/Dateibereich festlegen. Die Bridge führt keinen automatischen Rollback aus; Undo wird gezielt aufgerufen und seine Wirkung geprüft. Eine verlorene Antwort führt zur Fehlermeldung und erneuten Zustandsinspektion, nicht zum Wiederholen des Befehls.

Ergebnisse und Build-Grenzen: siehe `TEST-RESULTS.md`. Native Tests müssen vor einer Freigabe tatsächlich erfolgreich laufen; ein angelegter Workflow zählt noch nicht als Testergebnis.
