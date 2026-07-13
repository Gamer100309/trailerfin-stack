# Trailerfin Stack

Ein vollständiger, startklarer Docker-Stack: **Jellyfin + Prowlarr + Sonarr + Radarr + Lidarr + Jellyseerr + Trailerfin + ytdlp2STRM**.

Trailerfin sorgt automatisch für **Hintergrund-Trailer** (Cinema-Mode) in deiner Jellyfin-Bibliothek — über TMDb (offizielle API, kein Scraping) und einen YouTube-Link-Resolver (kein Speicherplatz nötig, oder wahlweise echter `.mp4`-Download).

> Basiert auf [Trailerfin: Rust](https://github.com/iPromKnight/trailerfin_rust) (iPromKnight), einer Neuentwicklung des ursprünglichen [Trailerfin](https://github.com/Pukabyte/trailerfin) (Pukabyte), sowie [ytdlp2STRM](https://github.com/fe80Grau/ytdlp2STRM) (fe80Grau). Dieser Fork behebt IMDb-Scraping-Blockaden, YouTube-Anti-Bot-Probleme, und ergänzt Sprachpräferenz, Sender-Whitelist/Blacklist und flexible Namenserkennung.

---

## Schnellstart

**Voraussetzung:** Docker + Docker Compose installiert.
```bash
curl -fsSL https://get.docker.com | sh
```

**1. Repository klonen**
```bash
git clone https://github.com/DEIN-USERNAME/trailerfin-stack.git
cd trailerfin-stack
```

**2. Konfiguration anlegen**
```bash
cp .env.example .env
nano .env
```
Trag mindestens ein:
- `TMDB_API_KEY` — kostenlos auf [themoviedb.org](https://www.themoviedb.org/settings/api) erstellen (Nutzungsart „Personal")
- `MEDIA_PATH` — wo deine Filme/Serien liegen sollen (Standard: `./media`, wird automatisch angelegt)

**3. Ordnerstruktur vorbereiten**
```bash
mkdir -p "${MEDIA_PATH:-./media}"/{Filme,Serien,Musik,downloads/complete,downloads/incomplete}
touch ytdlp2strm-configs/cookies.txt   # falls noch nicht vorhanden
```

**4. Starten**
```bash
docker compose up -d --build
```

Das dauert beim ersten Mal ein paar Minuten (Rust- und Deno-Build). Danach läuft alles unter:

| Dienst | URL |
|---|---|
| Jellyfin | `http://<SERVER-IP>:8096` |
| Prowlarr | `http://<SERVER-IP>:9696` |
| Sonarr | `http://<SERVER-IP>:8989` |
| Radarr | `http://<SERVER-IP>:7878` |
| Lidarr | `http://<SERVER-IP>:8686` |
| Jellyseerr | `http://<SERVER-IP>:5055` |

---

## Radarr/Sonarr Einrichtung (wichtig für Trailerfin!)

Damit Trailerfin deine Filme/Serien findet, müssen die **Ordnernamen ein TMDb-Tag** enthalten. Erkannt werden alle Varianten — Klammertyp, Reihenfolge und ob mit oder ohne "id" ist egal:

```
Der Pate (1972) [tmdbid-238]
Der Pate (1972) {tmdb-238}
Der Pate (1972) {tmdb-238} [imdbid-tt0068646]
```

**In Radarr:** Settings → Media Management → „Movie Folder Format":
```
{Movie Title} ({Release Year}) [imdbid-{ImdbId}] {tmdb-{TmdbId}}
```

**In Sonarr:** Settings → Media Management → „Series Folder Format":
```
{Series TitleYear} [imdbid-{ImdbId}] [tvdbid-{TvdbId}] {tmdb-{TmdbId}}
```

Für **bereits vorhandene** Filme/Serien: alle auswählen → „Edit" → Root Folder erneut setzen → „Apply Changes" (stößt ein Rename/Move an).

---

## Konfigurationsoptionen (`.env`)

| Variable | Beschreibung |
|---|---|
| `TRAILERFIN_MODE` | `link` (Standard, kein Speicherplatz) oder `download` (echte `.mp4`-Datei) |
| `TRAILERFIN_PREFERRED_LANGUAGE` | z.B. `de-DE` — fällt automatisch auf Standard zurück, falls kein Trailer in dieser Sprache existiert |
| `TRAILERFIN_CHANNEL_WHITELIST` | Kommagetrennte Liste bevorzugter YouTube-Kanäle (Priorität = Reihenfolge), z.B. offizielle Studio-Kanäle |
| `TRAILERFIN_CHANNEL_BLACKLIST` | Kommagetrennte Liste ausgeschlossener Kanäle |
| `TRAILERFIN_SCHEDULE` | Cron-Ausdruck für automatische Scans (Standard: täglich um Mitternacht) |

---

## Optional: Altersbeschränkte Trailer freischalten

Manche YouTube-Trailer sind altersbeschränkt und ohne Login nicht abrufbar. Um das zu umgehen:

1. Browser-Erweiterung **„Get cookies.txt LOCALLY"** installieren
2. Bei `youtube.com` einloggen (**Empfehlung:** dediziertes Wegwerf-Google-Konto nutzen, nicht dein Hauptkonto — die Datei enthält eine aktive Login-Sitzung)
3. Cookies exportieren → Datei ersetzt `ytdlp2strm-configs/cookies.txt`
4. `docker compose restart ytdlp2strm trailerfin`

Ohne Cookies funktioniert das System weiterhin einwandfrei für alle nicht-altersbeschränkten Trailer (die große Mehrheit).

---

## Bekannte Einschränkungen

- **Gelegentliche 403-Fehler beim Download-Modus:** YouTube/Google ändert seine Anti-Bot-Maßnahmen fortlaufend. Einzelne Trailer können zeitweise fehlschlagen — sie werden automatisch beim nächsten geplanten Lauf erneut versucht (kein manuelles Eingreifen nötig).
- **qBittorrent ist standardmäßig deaktiviert** (auskommentiert in der `docker-compose.yml`) — aktiviere es erst, wenn ein VPN vorgeschaltet ist.
- **`TRAILERFIN_CHANNEL_WHITELIST`/`BLACKLIST`** verursachen zusätzliche Anfragen an YouTubes oEmbed-Endpoint pro Trailer-Kandidat — das verlangsamt Scans geringfügig, ist aber unproblematisch für den täglichen Cron-Lauf.

---

## Neustart / Aktualisierung nach Code-Änderungen

Falls du `trailerfin_rust/` oder `ytdlp2strm/` selbst anpasst:
```bash
docker compose up -d --build
```

## Troubleshooting

**Trailerfin findet keine Ordner:** Prüfe das Namensformat (siehe oben) und ob `MOVIE_FOLDER_NAME`/`TV_FOLDER_NAME` in der `.env` mit deiner tatsächlichen Ordnerstruktur übereinstimmen.

**„Connection refused" zu ytdlp2strm:** Container startet noch — kurz warten und `docker compose restart trailerfin` erneut ausführen.

**Trailer spielt in Jellyfin nicht ab (Link-Modus):** `docker logs ytdlp2strm` prüfen — häufigste Ursache sind altersbeschränkte Videos (siehe Abschnitt oben) oder eine noch nicht vollständig gestartete `ytdlp2strm`-Instanz.
