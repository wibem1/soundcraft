# SoundCraft AI Studio — Entwicklungsplan

## Phase 0 — Referenz sichern
Upstream-Version, Commit und lokale Build-Voraussetzungen dokumentieren. Original-`main` unangetastet lassen.

## Phase 1 — Compatibility Manager und AI Bridge
Dokumentierte Befehle inventarisieren; isolierte deterministische Smoke-Tests; API-Diff und Diagnoseprotokoll. Keine externe KI notwendig.

## Phase 2 — Natives KI-Panel
Ein- und ausblendbares egui-Fenster mit minimalen Änderungen an SoundCraft. Projektzustand lesen und überprüfbare Befehle ausführen.

## Phase 3 — KI-Anbieter
OpenAI, Anthropic und Google als getrennte Adapter; sichere API-Schlüssel, Kosten, Dialog- und Aktionsverlauf.

## Phase 4 — Musikalische Assistenz
Kontextbewusste MIDI-Bearbeitung, Artikulation, Instrumentierung, Arrangement, später Mix und Audioanalyse.

## Verpflichtend bei jeder Phase
Reproduzierbare Tests und Diagnose, macOS/Windows-Verifikation, keine Änderungen am Audio-Kern, letzte funktionierende Fassung behalten. Änderungen am Upstream nur nach nachgewiesenem Nutzen übernehmen.

Status 2026-10-13: Phase 1 als getrenntes Rust-Modul implementiert. Bridge, API-Diff, isolierter Selbsttest und Diagnose sind lokal geprüft; native Plattform-/Desktop-Release-Gates bleiben offen. Siehe [Phase 1](PHASE-1.md) und [Testergebnisse](TEST-RESULTS.md).

Phase 2, 2026-10-13: natives Panel 0.1.1 implementiert; lokale UI-Tests und Bildkontrolle bestanden. Plattform-Builds im erweiterten CI-Workflow vorgesehen. Noch keine Provider-Anbindung. Siehe [Phase 2](PHASE-2.md).
