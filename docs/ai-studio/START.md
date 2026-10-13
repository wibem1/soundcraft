# SoundCraft AI Studio 0.2.0

1. ZIP vollständig entpacken. Unter Windows `SoundCraft-AI-Studio.exe` starten.
2. `Window > AI Studio` öffnen. Anbieter und Modell wählen.
3. Eigenen API-Schlüssel ins verdeckte Schlüsselfeld eingeben. `Für diese Sitzung verwenden` oder unter Windows/macOS `Sicher speichern` wählen.
4. Frage oder Auftrag eingeben und `Senden` klicken.

Beispiel: „Welche Spuren enthält mein Projekt, und was wäre ein sinnvoller nächster Schritt?“

Für eine Bearbeitung `Bearbeitungsvorschläge zulassen` aktivieren, beispielsweise:
„Lege eine Instrumentenspur namens Keys an, erzeuge einen vier Sekunden langen MIDI-Clip und füge eine C-Dur-Figur ein. Verwende den eingebauten Synthesizer.“

Die KI antwortet im Dialog und kann einen Vorschlag liefern. Erst `Vorschlag anwenden und prüfen` verändert das Projekt. Die App prüft den Vorschlag zuerst in einer getrennten Projektkopie, führt ihn dann über SoundCrafts Befehle aus und prüft das Ergebnis. `KI-Bearbeitung rückgängig` nimmt die unmittelbar letzte KI-Bearbeitung zurück. Nach weiteren manuellen Änderungen die normale Undo-Funktion verwenden. Aufträge an Dateien, Aufnahme oder externe Plugins sind in dieser Version nicht erlaubt.

Die Anbieter benötigen einen API-Schlüssel mit Modellzugang und gegebenenfalls API-Guthaben. Ein ChatGPT-, Claude- oder Gemini-Chat-Abonnement ist kein API-Schlüssel. Schlüssel nicht in Chatnachrichten einfügen.

Nachrichten und Projektübersicht einschließlich begrenzter MIDI-Daten werden zum gewählten Anbieter übertragen. Audiodateien und Dateipfade werden nicht mitgesendet. Diese Version hört das Projekt nicht ab und analysiert kein Audio. Pro Anfrage sind höchstens acht MIDI-Clips mit jeweils 256 Noten im Kontext enthalten.

Unter Windows/macOS können Schlüssel im nativen Betriebssystemspeicher liegen. Unter Linux gelten Schlüssel nur für die laufende Sitzung. Dialoge gespeicherter Projekte werden lokal im SoundCraft-Einstellungsordner unter `AI-Studio` abgelegt; ungespeicherte Projekte behalten ihren Dialog nur in der Sitzung. `Neuer Dialog` löscht den aktuellen Verlauf. Diagnose enthält standardmäßig nur technische Metadaten und Verbrauch; Dialog muss ausdrücklich eingeschlossen werden.

Tokenverbrauch stammt aus der Antwort des Anbieters. Kosten sind berechnete Standardtarife für bekannte Modelle, keine Rechnung. Gratisstufen, Kontoverträge und künftige Tarifänderungen können abweichen. Unbekannte Tarife werden gekennzeichnet. Lange OpenAI-Kontexte mit Sondertarifen sind durch die begrenzte Anfragegröße normalerweise ausgeschlossen; die Kontorechnung bleibt maßgeblich.

Entwicklungsversion: API-Protokolle und Bearbeitungen werden mit kontrollierten Anbieterantworten und lokalem HTTP-Transport geprüft. Ohne API-Schlüssel wurde kein erfolgreicher Live-Anbieteraufruf behauptet.
