# Phase 3 — nativer KI-Chat und geprüfte Bearbeitungen

SoundCraft AI Studio 0.2.0 ergänzt echte, nicht simulierte HTTPS-Anfragen an OpenAI Responses, Anthropic Messages und Google Gemini generateContent. API-Schlüssel werden ausschließlich als Authentifizierungsheader übertragen. Keine frei wählbaren Server, keine Weiterleitungen, keine automatischen Wiederholungen, begrenzte Anfrage/Antwortgröße, 10 Sekunden Verbindungs- und 180 Sekunden Gesamtzeitlimit. Netzwerk läuft in einem Hintergrundthread.

## Funktionsumfang

- Native Anbieterauswahl, editierbare Modell-ID, verdeckte Schlüssel, Sitzung oder nativer Windows/macOS-Speicher.
- Dialog mit aktueller Projektübersicht und begrenzten MIDI-Noten. Projektpfade werden entfernt. Kein Audio-Upload und keine behauptete Audioanalyse.
- Geprüfte Vorschläge für `track.new`, `midi.new_clip`, `midi.note_add`, `midi.note_edit`, `mix.volume`; maximal 128 Schritte. Neue Instrumentenspuren verwenden ausschließlich den eingebauten Synthesizer.
- Explizites Anwenden, Prüfung in isolierter Projektkopie, anschließend Ausführung über die dokumentierte Bridge und Prüfung von Spur-/Clip-/Noten-/Pegelzustand. Bei Fehler Undo und Prüfung der Wiederherstellung.
- Vorschläge besitzen Projektgeneration und Revisionsnummer. Projektwechsel oder zwischenzeitliche Bearbeitung verhindert Anwendung. KI-Antworten führen niemals selbsttätig Befehle aus.
- Letzte unmittelbar folgende KI-Bearbeitung gemeinsam rückgängig; bei manuellen Änderungen normale Undo-Funktion.
- Lokaler Dialog je gespeichertem Projekt, Kostenbelege, Diagnose ohne Dialoginhalt als Standard. Schlüssel sind weder serialisierbar noch im Dialogspeicher enthalten. Linux verwendet ausschließlich Sitzungsschlüssel, keinen scheinbar sicheren Mock-Speicher.

## Tests und Grenzen

Lokale Tests prüfen alle drei Anfrage-/Antwortprotokolle, Cache- und Thinking-Verbrauch, tatsächliche HTTP-Authentifizierungsheader mit Loopback-Server, redigierte Fehler, abgewiesene Weiterleitung/ungültige JSON-Antwort, tatsächliche MIDI-Bearbeitungen, Fehler-Rollback, Kontext ohne Pfade, Projektwechsel, veraltete Vorschläge und den vollständigen nativen Chat → Vorschlag → Apply → Undo-Ablauf. Anbieterantworten in diesen Tests sind ausdrücklich Fixtures.

Kein API-Schlüssel steht der Entwicklungsumgebung zur Verfügung. Deshalb keine erfolgreiche Live-Anbieterprüfung und keine Zusage über den Modellzugang eines Nutzerkontos. Die produktiven Aufrufe verwenden reale öffentliche API-Endpunkte; Nutzer gibt seinen eigenen Schlüssel in der App ein. Native Windows/macOS-Builds und OS-Speicherprüfung laufen separat im GitHub-Workflow. Ergebnisse werden erst nach tatsächlichem Abschluss dokumentiert.

## Quellen und Tarifstand 13.10.2026

- OpenAI Responses: https://developers.openai.com/api/docs/guides/text
- OpenAI Tarife: https://developers.openai.com/api/docs/pricing
- Anthropic Messages: https://platform.claude.com/docs/en/api/messages/create
- Anthropic Tarife: https://platform.claude.com/docs/en/about-claude/pricing
- Gemini generateContent: https://ai.google.dev/api/generate-content
- Gemini Modelle: https://ai.google.dev/gemini-api/docs/models
- Gemini Tarife: https://ai.google.dev/gemini-api/docs/pricing
- Native Schlüsselablage: https://docs.rs/keyring/3.6.3/keyring/

Kosten sind Standard-Texttarife in USD je Million Tokens. Hinterlegt sind OpenAI gpt-6.1-sol/gpt-6-sol 2 / 0,10 Cache / 10; gpt-6-luna 0,10 / 0,01 / 0,50; Claude Sonnet 5.5 2 / 0,10 / 10; Opus 5.5 4 / 0,20 / 20. Anthropic Cache-Schreiben kostet 1,25× Eingabe für 5 Minuten und 2× für 1 Stunde. Gemini 3.8/3.7 Flash 0,75 / 0,075 / 3,75 bis 31.12.2026, danach das Doppelte; Gemini 3.5 Flash Lite 0,30 / 0,03 / 2,50. Unbekannte Modelle und fehlende Verbrauchsdaten ergeben keinen erfundenen Betrag. Thinking-Tokens werden als Teil der Ausgabe gerechnet, bei Gemini zusätzlich zu candidatesTokenCount. Die Anzeige ist keine Kontorechnung.

## Architektur

Alle neuen Provider-, Secret-, History- und Planmodule liegen im getrennten `crates/ai-studio`; nativer Dialogadapter in `crates/ui-egui/src/ai_chat.rs`. Unveränderte Engine/DSP/Playback/Plugin-Host-Quellen. Einstellungen und Projektwechselhaken folgen bestehenden UI-/Command-Pfaden. `main` wird nicht verändert.
