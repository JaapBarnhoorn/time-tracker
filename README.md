# 🕒 Time Tracker

Een moderne, lichtgewicht desktop applicatie voor het bijhouden van je werkzaamheden. Gebouwd met **Svelte 5**, **Tauri 2**, en **Rust**. 

Deze applicatie is ontworpen om privacy-vriendelijk te zijn (alle data blijft lokaal op je eigen computer in een SQLite database) en helpt je bij de administratie van je uren door middel van slimme automatisering.

## ✨ Belangrijkste Functionaliteiten

- **Real-time Tracking:** Start en stop taken met één klik. Pas achteraf eenvoudig de starttijd aan als je bent vergeten de timer aan te zetten.
- **Slimme Scheduler:** Plan terugkerende taken in (dagelijks, wekelijks, 2-wekelijks of maandelijks).
    - **Reminders:** Krijg 5 minuten van tevoren een melding dat er een taak aankomt.
    - **Auto-Switch:** De app stopt automatisch je huidige activiteit en start de geplande taak op het juiste moment.
- **Weekoverzicht:** Een speciaal rapportagescherm dat uren per werksoort per dag groepeert. Ideaal voor het overtypen van uren aan het einde van de week.
- **Privacy First:** Geen cloud, geen accounts. Je data staat veilig in een lokale SQLite database.
- **Sneltoetsen (Keyboard First):**
    - `/` : Snel zoeken in de volledige takenlijst.
    - `1` t/m `9` : Start direct een van je top-9 meest gebruikte taken.
    - `R` : Hervat je laatste activiteit.
    - `S` : Stop de huidige timer.
    - `,` : Open instellingen.
    - `+` : Handmatig een uren-record toevoegen.
- **Cross-platform:** Werkt op macOS, Windows en Linux.

## 🚀 Installatie

### Voor gebruikers
Download de nieuwste versie van de [Releases pagina](https://github.com/JaapBarnhoorn/time-tracker/releases).
- **macOS:** Gebruik het `.dmg` bestand.
- **Windows:** Gebruik de `.exe` installer.
- **Linux:** Gebruik het `.deb` pakket (voor Ubuntu) of de `.AppImage`.

### Voor ontwikkelaars
1. Clone de repository.
2. Installeer afhankelijkheden: `pnpm install`.
3. Start de app in development modus: `pnpm tauri dev`.

## 🛠️ Configuratie & Data Import

De applicatie wordt geleverd met een schone lei (geen vooringestelde taken). Om snel te starten met een lijst van werksoorten:

1. Zorg dat je een `.json` bestand hebt met een lijst van namen (bijv. `seed_tasks.json`).
2. Open de app en ga naar **Instellingen** (tandwiel icoon of toets `,`).
3. Scroll naar **Data Beheer**.
4. Klik op **Taken importeren (.json)** en selecteer je bestand.

## 💻 Techniek

- **Frontend:** Svelte 5 (met modernste Reactivity API / Signals).
- **Styling:** Tailwind CSS.
- **Backend:** Rust met Tauri 2.0.
- **Database:** SQLite (lokaal opgeslagen in de app-data map van je OS).
- **CI/CD:** GitHub Actions voor automatische cross-platform builds.

## 📄 Licentie

Dit project is beschikbaar onder de MIT licentie.
