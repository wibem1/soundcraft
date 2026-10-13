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

Status: Nur technische Dokumente; keine Erweiterung implementiert oder getestet.
