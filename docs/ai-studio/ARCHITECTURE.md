# SoundCraft AI Studio — Architektur und Anforderungen

Stand: 2026-10-13. Projektstatus: technische Vorbereitungsphase.

## Leitidee
Ein jederzeit verfügbarer musikalischer KI-Assistent in SoundCraft — von Komposition, Instrumentierung und Arrangement über MIDI- und Audiobearbeitung bis zum Mix. Er berät oder führt auf ausdrücklichen Auftrag Änderungen durch.

## Plattformen
macOS und Windows gleichberechtigt. Gemeinsamer Rust-Code; plattformspezifische Anteile auf sichere Schlüsselablage, Installationspakete und notwendige OS-Anpassungen begrenzen.

## Architekturgrenzen
- SoundCraft bleibt die DAW; Audio-Engine, DSP, Wiedergabe und Plugin-Hosts werden zunächst nicht modifiziert.
- Der Assistent nutzt dokumentierte MCP- und JSON-Control-Schnittstellen über eine schmale AI Bridge.
- Keine direkten schreibenden Eingriffe des Assistenten in interne Session-Strukturen.
- KI-Provider, Dialog, Projektkontext, Bridge, Diagnose und GUI-Panel bleiben voneinander getrennt.
- Das native egui-Panel wird mit möglichst wenigen Integrationspunkten realisiert.
- Aktionen werden überprüft und, soweit von SoundCraft unterstützt, rückgängig gemacht; der Echtzeit-Audiothread bleibt frei von KI-Arbeit.
- Musikalischer Kontext und Gesprächsverlauf werden projektspezifisch verwaltet; Kosten transparent ausgewiesen.

## Qualitätsregeln
- Vor jedem Build: klare Versionsnummer, Datum, SoundCraft-Basiscommit und eigener Commit.
- Tests auf macOS und Windows vor Freigabe; keine ungeprüften Erfolgsmeldungen.
- Funktionierende Version niemals überschreiben; Testversionen separat erstellen.
- Diagnosen enthalten Modell, Anfrage-/Aktionstyp, API-Aufrufe, Fehler, Zeit und Kosten; keine Zugangsdaten; musikalische Daten nur nach Opt-in.
- Bei unbeherrschbaren Querverflechtungen, invasiven Kernänderungen oder unklaren Schnittstellen die Erweiterung stoppen statt Trial-and-Error.

## Dokumentation
- [API-Vertrag](API-CONTRACT.md)
- [Entwicklungsplan](ROADMAP.md)
