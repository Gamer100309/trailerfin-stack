# Jellyfin Plugin: Trailerfin

Ein natives Jellyfin-Plugin (C#/.NET), das automatisch Hintergrund-Trailer
(Cinema-Mode) für Filme und Serien erzeugt — über TMDb (offizielle API)
und einen YouTube-Link-Resolver ([ytdlp2STRM](https://github.com/fe80Grau/ytdlp2STRM)).

**Kein Ordnernamens-Tag nötig** — die TMDb-ID wird direkt aus Jellyfins
eigenen, bereits erkannten Metadaten gelesen (`item.ProviderIds["Tmdb"]`).

> Inspiriert von [Trailerfin: Rust](https://github.com/iPromKnight/trailerfin_rust)
> (iPromKnight), einer Neuentwicklung des ursprünglichen
> [Trailerfin](https://github.com/Pukabyte/trailerfin) (Pukabyte).
> Dies ist eine komplette Neuentwicklung als natives C#-Plugin.

## Installation

1. Jellyfin-Dashboard → Plugins → Repositories → „+"
2. Repository-URL: https://raw.githubusercontent.com/Gamer100309/trailerfin-stack/Trailerfin-Plugin/manifest.json
