# BookCore ⚡ v0.1

> High-performance, lightweight personal book, manga, and comic server written in 100% pure Rust with zero local C library runtime dependencies.

---

## Features

- **Blazing Fast Scanning & Parsers:**
  - **CBZ / ZIP:** Natural sort image ordering (`page1.jpg` < `page2.jpg` < `page10.jpg`), embedded `ComicInfo.xml` metadata extraction, fallback filename regex heuristics.
  - **EPUB:** Pure-Rust OPF package & spine parser, table of contents extractor, chapter HTML scoping, embedded CSS/image/font asset rewriter.
  - **PDF:** Native viewer integration with HTTP byte-range request streaming (`Accept-Ranges: bytes`, 206 Partial Content).
- **Embedded Web Reader UI:**
  - Responsive single-page application served directly from port `3435`.
  - Dark mode by default, customizable typography (size, line-height, Sepia/Dark/Light themes).
  - Comic/Manga reader with Fit to Height, Fit to Width, touch zone page flipping, and scrubber.
  - EPUB reader with chapter navigation and book asset streaming.
- **Reading Timer & Habits Engine:**
  - Automated Pomodoro focus cycles (25m focus / 5m break) with synthesized Web Audio chimes (zero external mp3 assets).
  - Multi-stage activity detection: `visibilitychange` tab blur pausing, 60s inactivity timeout.
  - Real-time 15s heartbeats with Pages-Per-Minute (PPM) telemetry.
  - Daily reading streaks and weekly analytics bar chart.
  - Bookmarks drawer with one-click page jumps.
- **Zero-Local DevOps:**
  - Multi-stage Docker container build (<30MB).
  - GitHub Actions CI/CD with automated `cargo check`, `cargo test`, `cargo clippy`, and GHCR release.

---

## Quickstart with Docker Compose

```bash
docker compose up -d
```

Access the reader at [http://localhost:3435](http://localhost:3435).

---

## Configuration

| Environment Variable | Default | Description |
|---|---|---|
| `PORT` | `3435` | HTTP server port |
| `BOOKCORE_DB_PATH` | `/data/bookcore.db` | SQLite database file location |
| `BOOKCORE_BOOKS_DIR` | `/data/books` | Root directory for auto-scanning books |
| `BOOKCORE_CACHE_DIR` | `/data/cache` | Directory for extracted covers and cache |
| `RUST_LOG` | `bookcore=info,tower_http=info` | Tracing log level filter |

---

## REST API Overview

- `GET /` — Serves embedded Single-Page Reader Application
- `GET /api/health` — Service health status
- `GET /api/info` — Server metadata and storage counts
- `POST /api/scan` — Scan library files
- `GET /api/books` — List and filter books (`format`, `query`, `library_id`, `limit`, `offset`)
- `GET /api/books/:id` — Detailed book record with spine/chapters
- `DELETE /api/books/:id` — Remove book from database
- `GET /api/books/:id/cover` — Book cover image
- `GET /api/books/:id/pages/:page_num` — CBZ archive image page
- `GET /api/books/:id/chapters` — EPUB spine chapter list
- `GET /api/books/:id/chapters/:chapter_idx` — Scoped EPUB chapter HTML
- `GET /api/books/:id/assets?file=...` — EPUB embedded assets
- `GET /api/books/:id/raw` — Raw PDF stream with byte-range support
- `GET /api/books/:id/progress` & `POST /api/books/:id/progress` — Reading progress
- `GET /api/books/:id/bookmarks` & `POST /api/books/:id/bookmarks` — Bookmarks
- `POST /api/timer/start`, `heartbeat`, `pause`, `stop` — Reading session telemetry
- `GET /api/timer/stats` — Streak counts, PPM, and historical reading charts
