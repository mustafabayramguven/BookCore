# BookCore v0.1 Specification & Kavita Reverse-Engineering Blueprint

**Application Name:** `BookCore`  
**Binary / Crate Name:** `bookcore`  
**Version:** `0.1.0` (`v0.1`)  
**Default Network Port:** `3435` (hardcoded default in server, `Dockerfile`, and `docker-compose.yml`)  
**Target Environment:** Linux / Docker (pure Rust, SQLite embedded, zero external C library runtime dependencies)

---

## 1. Evidence-Backed Architecture of Kavita Core Reading Features

Static analysis was conducted directly on the Kavita repository (`/home/mustafa/Kavita`). Below is the architectural reverse-engineering of Kavita's core scanning, parsing, reading, progress tracking, and session calculation systems, backed by exact source paths, classes, and methods.

### 1.1 Solution & Project Architecture
Kavita is structured into several C# .NET projects:
* **`Kavita.Models`**: Core domain entities and DTOs (`Kavita.Models/Entities/` and `Kavita.Models/DTOs/`).
  * `Library.cs`, `Series.cs`, `Volume.cs`, `Chapter.cs`, `MangaFile.cs` form the 5-tier hierarchical entity graph.
  * `AppUserProgress.cs`, `AppUserBookmark.cs`, `AppUserReadingSession.cs`, `AppUserReadingSessionActivityData.cs` manage user state.
* **`Kavita.Services`**: Business logic, file scanning, format extraction, and stats calculation.
  * `Scanner/ScannerService.cs`, `Scanner/ParseScannedFiles.cs`, `Scanner/Parser.cs`, `Scanner/BookParser.cs`, `Scanner/DefaultParser.cs`.
  * `ArchiveService.cs`, `BookService.cs`, `CacheService.cs`, `Reading/ReaderService.cs`, `Reading/ReadingSessionService.cs`, `StatisticService.cs`.
* **`Kavita.Server`**: ASP.NET Core web host and API controllers.
  * `Controllers/LibraryController.cs`, `Controllers/BookController.cs`, `Controllers/ReaderController.cs`, `Controllers/StreamController.cs`, `Controllers/StatsController.cs`.
* **`Kavita.Database`**: Entity Framework Core DbContext (`DataContext.cs`) and SQLite migrations.

---

### 1.2 Library Scanning & Folder Discovery
* **File:** `Kavita.Services/Scanner/ScannerService.cs` (`ScanLibrary`, `ScanLibraryFolders`, `ProcessParsedSeries`, `RemoveSeriesNotFound`)
* **File:** `Kavita.Services/Scanner/ParseScannedFiles.cs` (`ScanFiles`, `ScanDirectories`, `ScanSingleDirectory`)
* **Mechanism:**
  1. **Folder Traversal:** `ScannerService.ScanLibrary(int libraryId, bool forceUpdate)` reads the library root directories configured in `Library.Folders`. `ParseScannedFiles.ScanFiles` recursively enumerates directories using `DirectoryService.ListFolderTreeAsync`, matching allowed extensions against regex patterns for CBZ, ZIP, EPUB, PDF, and raw image files (`Parser.SupportedExtensions`).
  2. **Hierarchy Construction:** Files are grouped by directory and series name. For loose files or subdirectories, Kavita checks if folders represent Series, Volumes, or Chapters.
  3. **Diff & Tombstone Removal:** In Step 5 of `ScanLibraryFolders` (`RemoveSeriesNotFound`), any `Series`, `Volume`, or `Chapter` that previously existed in the database for the given library but was not matched on disk is deleted via `SeriesRepository.RemoveSeriesNotInListAsync`.

---

### 1.3 Filename Parsing, Regex Heuristics & ComicInfo
* **File:** `Kavita.Services/Scanner/Parser.cs`
* **File:** `Kavita.Services/Scanner/DefaultParser.cs` & `Kavita.Services/Scanner/BookParser.cs`
* **File:** `Kavita.Models/Metadata/ComicInfo.cs`
* **Mechanism:**
  1. **ComicInfo.xml Priority:** For comic archives (`.cbz`, `.zip`), Kavita opens the archive and searches for `ComicInfo.xml` (case-insensitive) via `ArchiveService.GetComicInfo(string archivePath)`. If present, `ComicInfo` supplies:
     * `Title`, `Series`, `Number` (Chapter index/number), `Volume`, `Summary`, `Genre`, `PageCount`, `LanguageISO`, `Year/Month/Day`.
     * If `ComicInfo.Series` exists, it overrides filename heuristics.
  2. **Filename Regex Heuristics:** If no `ComicInfo.xml` exists, `Parser.cs` evaluates an ordered array of regexes (`MangaSeriesRegex`):
     * Matches patterns such as:
       * `(?<Series>.+?)(\s|_|-)+(?:Vol(ume|\.)?(\s|_|-)+\d+)(\s|_|-)+(?:(Ch|Chapter|Ch)\.?)(\s|_|-)+(?<Chapter>\d+)`
       * `(?<Series>.*)(\b|_)v(?<Volume>\d+-?\d*)(\s|_|-)`
       * `(?<Series>.*) (\b|_|-)(v|ch\.?|c|s)\d+`
     * Normalizes series names (`Parser.NormalizeTitle`), strips brackets (`[ScanGroup]`, `(2021)`), trims zero-padded numbers, and assigns `Parser.DefaultChapter = "0"` or `Parser.LooseLeafVolume = "0"` if absent.

---

### 1.4 Archive & CBZ/ZIP Image Extraction
* **File:** `Kavita.Services/ArchiveService.cs` (`CanOpen`, `GetNumberOfPagesFromArchive`, `GetCoverImage`, `FirstFileEntry`)
* **File:** `Kavita.Services/CacheService.cs` (`Ensure`, `ExtractChapterFiles`, `GetCachedPagePath`)
* **File:** `Kavita.Server/Controllers/ReaderController.cs` (`GetImage`, `GetThumbnail`)
* **Mechanism:**
  1. **Archive Inspection:** `ArchiveService.CanOpen` attempts reading via `System.IO.Compression.ZipFile`. It counts image files matching `Parser.ImageFileExtensions` (`.png`, `.jpeg`, `.jpg`, `.webp`, `.gif`, `.avif`), excluding system junk (`__MACOSX`, `._*`, hidden thumbs).
  2. **Page Ordering:** Archive entries are sorted using a natural sort comparer (`OrderByNatural`). Page indices are 0-based (`0 .. N-1`).
  3. **Cover Extraction:** `FindCoverImageFilename` inspects filenames containing `cover` or `folder`, or falls back to `FirstFileEntry` (page 0).
  4. **Reading / Cache Pipeline:** In `ReaderController.GetImage(chapterId, page, apiKey)`:
     * `CacheService.Ensure(chapterId)` unpacks archive images into a temporary cache folder: `config/cache/chapters/{chapterId}/`.
     * If already cached, `GetCachedPagePath(chapterId, page)` returns the cached file stream directly with `Cache-Control: public, max-age=3600`.

---

### 1.5 EPUB Parsing, Spine, HTML Scoping & Asset Serving
* **File:** `Kavita.Services/BookService.cs` (`ParseInfo`, `GetBookPage`, `ScopePage`, `GetResourceAsync`, `GenerateTableOfContents`)
* **File:** `Kavita.Server/Controllers/BookController.cs` (`GetBookInfo`, `GetBookChapters`, `GetBookPage`, `GetBookPageResources`)
* **Mechanism:**
  1. **EPUB Metadata & Spine:** An EPUB file is a ZIP archive containing `META-INF/container.xml`. That points to the `OPF` package file (e.g., `OEBPS/content.opf`).
     * `manifest`: lists item IDs, hrefs, and media-types.
     * `spine`: defines the linear reading order (`itemref idref="..."`).
     * `toc.ncx` / EPUB 3 Navigation Document (`nav.xhtml`): defines the Table of Contents hierarchy.
     * `calibre:series` and `calibre:series_index` / `belongs-to-collection` are extracted for book series ordering.
  2. **Page Scoping Pipeline (`BookService.ScopePage`):**
     * Instead of raw rendering, Kavita parses each spine XHTML document into an HTML DOM using `HtmlAgilityPack`.
     * `ScopeImages`: Rewrites all `<img src="...">` and `<image xlink:href="...">` paths to point to `/api/book/{chapterId}/book-resources?file={encodedPath}`.
     * `InlineStyles` & `ScopeStyles`: Scopes CSS rules to prevent book styles from leaking into the parent reader application shell.
     * `RewriteAnchors`: Maps internal anchor hrefs to the respective reading page index using a precomputed key-to-page map.
  3. **Resource Streaming:** `BookController.GetBookPageResources` retrieves embedded stylesheets, fonts (`.woff2`, `.ttf`), and images directly from the EPUB ZIP container by key and streams them with appropriate MIME types.

---

### 1.6 PDF Handling & Streaming
* **File:** `Kavita.Services/BookService.cs` (`ExtractPdfImages`)
* **File:** `Kavita.Server/Controllers/ReaderController.cs` (`GetPdf`)
* **File:** `Kavita.Server/Controllers/StreamController.cs`
* **Mechanism:**
  1. **Raw Streaming:** `ReaderController.GetPdf` serves the binary PDF file with HTTP range request support (`Accept-Ranges: bytes`, `Content-Type: application/pdf`, `Content-Disposition: inline`).
  2. **Rasterization Alternative:** In Kavita, `ExtractPdfImages` utilizes DocNet/PDFium to rasterize PDF pages to PNGs for clients without native PDF viewers (like Mihon). In BookCore, standard native embedded PDF viewing (`<embed>` / PDF.js) and direct raw streaming is used.

---

### 1.7 Reading Progress & Bookmarks Tracking
* **File:** `Kavita.Models/Entities/Progress/AppUserProgress.cs`
* **File:** `Kavita.Models/Entities/User/AppUserBookmark.cs`
* **File:** `Kavita.Services/Reading/ReaderService.cs` (`SaveReadingProgress`, `GetContinuePoint`)
* **File:** `Kavita.Server/Controllers/ReaderController.cs` (`GetProgress`, `SaveProgress`, `GetBookmarks`, `Bookmark`, `Unbookmark`)
* **Mechanism:**
  1. **Progress Entity:** Stores `PagesRead` (current 0-based page), `BookScrollId` (nearest EPUB DOM anchor / XPath for continuous reading), `TotalReads` (how many times completed), `LastModifiedUtc`.
  2. **Continue Point Calculation:** `ReaderService.GetContinuePoint` resolves the next unread chapter in a series, or resumes the in-progress chapter.
  3. **Bookmarks:** Stores `Page`, `ChapterId`, `XPath` (for EPUB), `ChapterTitle`, `CreatedUtc`.

---

### 1.8 Reading Sessions & Stats Engine
* **File:** `Kavita.Models/Entities/Progress/AppUserReadingSession.cs`
* **File:** `Kavita.Models/Entities/Progress/AppUserReadingSessionActivityData.cs`
* **File:** `Kavita.Services/Reading/ReadingSessionService.cs`
* **File:** `Kavita.Services/StatisticService.cs` (`GetReadingPaceForUser`, `reading-activity`)
* **Mechanism:**
  1. **Session Ingestion:** In `ReadingSessionService`, progress updates feed into an active session. If a session is older than `sessionTimeout` (default 10 minutes) without activity, it is finalized (`EndTimeUtc = lastHeartbeat`), and a new session begins.
  2. **Session Aggregations:** `ReadingPaceDto` calculates:
     * Total reading duration: `Sum(SessionEnd - SessionStart)`.
     * Total pages read: `Sum(PagesRead)`.
     * Reading speed (PPM): `pages_read / active_minutes`.

---

## 2. Complete Rust Port Blueprint: BookCore v0.1 (Port 3435)

### 2.1 Crate & Workspace Layout
BookCore v0.1 is implemented as a single, modular Rust crate inside `/home/mustafa/agy/bookcore/`.

```
/home/mustafa/agy/bookcore/
├── Cargo.toml
├── Dockerfile
├── docker-compose.yml
├── .dockerignore
├── README.md
├── SPEC.md
├── data/                      # Default local runtime data directory (gitignored)
│   ├── books/                 # Sample reading library folder
│   ├── cache/                 # Extracted images / temporary cache
│   └── bookcore.db            # SQLite database file
├── src/
│   ├── main.rs                # Entrypoint: CLI args, config, DB init, Axum router on port 3435
│   ├── config.rs              # App configuration, port 3435, DB path, library dirs
│   ├── db/
│   │   ├── mod.rs             # Database connection pool & auto-migrations
│   │   ├── schema.rs          # SQL DDL schemas & table creators
│   │   └── queries/           # Typed queries for books, progress, bookmarks, sessions
│   │       ├── books.rs
│   │       ├── bookmarks.rs
│   │       ├── progress.rs
│   │       └── sessions.rs
│   ├── models/
│   │   ├── mod.rs
│   │   ├── book.rs            # Book, Chapter, Format (CBZ, EPUB, PDF)
│   │   ├── progress.rs        # ProgressDto, BookmarkDto
│   │   └── timer.rs           # Session, Heartbeat, Stats, Streak, PPM
│   ├── scanner/
│   │   ├── mod.rs             # Scanner orchestrator
│   │   ├── walk.rs            # Recursive directory walker & glob matcher
│   │   ├── archive.rs         # CBZ / ZIP reader, image listing, natural sort
│   │   ├── comic_info.rs      # ComicInfo.xml deserializer
│   │   ├── epub.rs            # EPUB container, OPF manifest/spine & TOC parser
│   │   └── pdf.rs             # PDF basic inspector & metadata extractor
│   ├── reader/
│   │   ├── mod.rs             # Reader content delivery engine
│   │   ├── cbz_reader.rs      # Archive page streaming & thumbnail cache
│   │   ├── epub_reader.rs     # Spine reading, XHTML rewriting & asset serving
│   │   └── pdf_reader.rs      # Range request PDF streaming
│   ├── timer/
│   │   ├── mod.rs             # Reading session state manager
│   │   ├── tracker.rs         # Active session heartbeat & auto-pause logic
│   │   └── streak.rs          # Streak and Pages-Per-Minute (PPM) calculator
│   ├── api/
│   │   ├── mod.rs             # Router aggregation
│   │   ├── health.rs          # /api/health and /api/info
│   │   ├── books.rs           # /api/scan, /api/books, /api/books/:id
│   │   ├── reader.rs          # Page images, EPUB chapters/assets, PDF raw
│   │   ├── progress.rs        # Reading progress CRUD
│   │   ├── bookmarks.rs       # Bookmarks CRUD
│   │   └── timer.rs           # /api/timer/start, heartbeat, pause, stop, stats
│   └── web/
│       ├── mod.rs             # Static asset router & embedded HTML templates
│       ├── index.html         # Single-Page Application (HTMX + Alpine.js + CSS)
│       └── static/
│           ├── app.css        # Clean, modern responsive reader & library styles
│           └── app.js         # Client-side idle detection, timer heartbeat, reader hotkeys
```

---

### 2.2 Pure-Rust Dependencies & Features (`Cargo.toml`)
To adhere to the requirement for reliable compilation without external C libraries (and zero native C library link failures), all dependencies are pure-Rust or bundle standard SQLite through `sqlx`.

```toml
[package]
name = "bookcore"
version = "0.1.0"
edition = "2021"
description = "High-performance lightweight personal book & comic reading server"
authors = ["BookCore Team"]

[dependencies]
# Async Runtime & HTTP Server
tokio = { version = "1.41", features = ["full"] }
axum = { version = "0.7", features = ["macros", "multipart"] }
tower = { version = "0.5", features = ["util"] }
tower-http = { version = "0.6", features = ["fs", "cors", "trace", "compression-gzip", "range-request"] }

# SQLite Database
sqlx = { version = "0.8", default-features = false, features = ["sqlite", "runtime-tokio", "macros", "chrono", "uuid"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Archives & Parsing (Pure Rust)
zip = { version = "2.2", default-features = false, features = ["deflate", "bzip2"] }
quick-xml = { version = "0.37", features = ["serialize"] }
roxmltree = "0.20"

# Utilities & System
walkdir = "2.5"
uuid = { version = "1.11", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
mime_guess = "2.0"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
regex = "1.11"
percent-encoding = "2.3"
```

---

### 2.3 SQLite Schema & Storage Specification
* **Database File:** Configurable via environment variable `BOOKCORE_DB_PATH`. Defaults to `/data/bookcore.db` (local dev fallback: `./data/bookcore.db`).
* **Parent Directory Creation:** The application startup creates the parent directory of `BOOKCORE_DB_PATH` if it does not exist (`std::fs::create_dir_all`).
* **SQLite PRAGMAs:**
  ```sql
  PRAGMA journal_mode = WAL;
  PRAGMA synchronous = NORMAL;
  PRAGMA foreign_keys = ON;
  PRAGMA busy_timeout = 5000;
  ```

#### Complete DDL Schema
```sql
-- 1. Libraries table
CREATE TABLE IF NOT EXISTS libraries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    path TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 2. Books table (Unified representation of Books, Manga, Comics)
CREATE TABLE IF NOT EXISTS books (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    library_id INTEGER NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    series TEXT NOT NULL DEFAULT '',
    volume TEXT NOT NULL DEFAULT '',
    chapter TEXT NOT NULL DEFAULT '',
    format TEXT NOT NULL, -- 'EPUB', 'CBZ', 'PDF'
    file_path TEXT NOT NULL UNIQUE,
    file_size INTEGER NOT NULL,
    page_count INTEGER NOT NULL DEFAULT 0,
    cover_image_path TEXT,
    summary TEXT,
    genre TEXT,
    language TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_books_library_id ON books(library_id);
CREATE INDEX IF NOT EXISTS idx_books_format ON books(format);
CREATE INDEX IF NOT EXISTS idx_books_series ON books(series);

-- 3. Book Chapters table (for EPUB spine / table of contents)
CREATE TABLE IF NOT EXISTS book_chapters (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    book_id INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    chapter_index INTEGER NOT NULL,
    title TEXT NOT NULL,
    content_href TEXT NOT NULL,
    word_count INTEGER NOT NULL DEFAULT 0,
    UNIQUE(book_id, chapter_index)
);
CREATE INDEX IF NOT EXISTS idx_book_chapters_book_id ON book_chapters(book_id);

-- 4. Reading Progress table
CREATE TABLE IF NOT EXISTS reading_progress (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    book_id INTEGER NOT NULL UNIQUE REFERENCES books(id) ON DELETE CASCADE,
    current_page INTEGER NOT NULL DEFAULT 0,
    scroll_anchor TEXT, -- XPath or DOM anchor for continuous EPUB reading
    is_finished BOOLEAN NOT NULL DEFAULT 0,
    total_reads INTEGER NOT NULL DEFAULT 0,
    last_read_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_reading_progress_book ON reading_progress(book_id);

-- 5. Bookmarks table
CREATE TABLE IF NOT EXISTS bookmarks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    book_id INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    page_number INTEGER NOT NULL,
    title TEXT,
    xpath TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_bookmarks_book ON bookmarks(book_id);

-- 6. Reading Sessions table (Reading Timer & Tracker)
CREATE TABLE IF NOT EXISTS reading_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    book_id INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    started_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_heartbeat_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    ended_at DATETIME,
    active_seconds INTEGER NOT NULL DEFAULT 0,
    start_page INTEGER NOT NULL DEFAULT 0,
    end_page INTEGER NOT NULL DEFAULT 0,
    pages_read INTEGER NOT NULL DEFAULT 0,
    is_pomodoro BOOLEAN NOT NULL DEFAULT 0,
    goal_minutes INTEGER NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT 1
);
CREATE INDEX IF NOT EXISTS idx_sessions_book ON reading_sessions(book_id);
CREATE INDEX IF NOT EXISTS idx_sessions_active ON reading_sessions(is_active);
CREATE INDEX IF NOT EXISTS idx_sessions_started ON reading_sessions(started_at);

-- 7. Daily Reading Stats table (Pre-aggregated rollups for instant streak & PPM queries)
CREATE TABLE IF NOT EXISTS daily_stats (
    date_str TEXT PRIMARY KEY, -- 'YYYY-MM-DD'
    total_seconds INTEGER NOT NULL DEFAULT 0,
    pages_read INTEGER NOT NULL DEFAULT 0,
    books_touched INTEGER NOT NULL DEFAULT 0,
    sessions_count INTEGER NOT NULL DEFAULT 0
);
```

---

### 2.4 Axum REST API Specifications

The server runs on port `3435`. All API endpoints use `/api/...` prefix and return standard JSON or media byte streams.

#### 2.4.1 System & Health Endpoints
* **`GET /api/health`**
  * **Response:** `200 OK`
  * ```json
    { "status": "ok", "app": "BookCore", "version": "0.1.0", "port": 3435 }
    ```
* **`GET /api/info`**
  * **Response:** `200 OK`
  * ```json
    {
      "name": "BookCore",
      "version": "0.1.0",
      "db_path": "/data/bookcore.db",
      "storage": { "total_books": 42, "total_libraries": 1 }
    }
    ```

#### 2.4.2 Library & Book Management Endpoints
* **`POST /api/scan`**
  * **Description:** Initiates asynchronous or synchronous scan of all configured library directories.
  * **Request Body (optional):**
    ```json
    { "library_path": "/data/books", "force_refresh": false }
    ```
  * **Response:** `200 OK`
    ```json
    {
      "status": "completed",
      "scanned_files": 128,
      "added_books": 12,
      "updated_books": 2,
      "removed_books": 0,
      "elapsed_ms": 345
    }
    ```
* **`GET /api/books`**
  * **Query Params:** `query` (search string), `format` (`EPUB`|`CBZ`|`PDF`), `library_id` (int), `limit`, `offset`.
  * **Response:** `200 OK`
    ```json
    [
      {
        "id": 1,
        "title": "Berserk Vol. 01",
        "series": "Berserk",
        "volume": "1",
        "chapter": "0",
        "format": "CBZ",
        "page_count": 224,
        "current_page": 45,
        "progress_percent": 20.08,
        "is_finished": false,
        "has_cover": true
      }
    ]
    ```
* **`GET /api/books/:id`**
  * **Response:** Detailed book record with chapter list, metadata, and progress.
* **`DELETE /api/books/:id`**
  * **Response:** `204 No Content` (removes database record; underlying file is preserved or optionally deleted).

#### 2.4.3 Reader Content Delivery Endpoints
* **`GET /api/books/:id/cover`**
  * **Description:** Returns the extracted cover image (WebP/JPEG/PNG) with HTTP cache headers.
* **`GET /api/books/:id/pages/:page_num`**
  * **Description:** Used for CBZ/ZIP manga and comic reading.
  * **Path Params:** `page_num` (0-indexed page number).
  * **Behavior:** Reads directly from archive entry or cached page, streams image with proper `Content-Type: image/jpeg|png|webp` and `Cache-Control: public, max-age=86400`.
* **`GET /api/books/:id/chapters`**
  * **Description:** Returns the Table of Contents / Spine chapters for EPUB books.
  * **Response:** `200 OK`
    ```json
    [
      { "chapter_index": 0, "title": "Cover", "content_href": "cover.xhtml" },
      { "chapter_index": 1, "title": "Chapter 1: The Gathering", "content_href": "chapter1.xhtml" }
    ]
    ```
* **`GET /api/books/:id/chapters/:chapter_idx`**
  * **Description:** Returns the scoped HTML page for an EPUB chapter.
  * **Behavior:** Extracts the XHTML file from the EPUB container, rewrites image/CSS asset URLs to `/api/books/:id/assets?file=...`, scopes typography, and serves as HTML.
* **`GET /api/books/:id/assets`**
  * **Query Params:** `file` (relative path inside EPUB ZIP container).
  * **Behavior:** Streams embedded images, fonts, and stylesheets from the EPUB container.
* **`GET /api/books/:id/raw`**
  * **Description:** Used for PDF streaming and raw file downloading.
  * **Headers:** Supports `Range: bytes=start-end` for instant seeking in PDF viewers. `Content-Type: application/pdf`, `Accept-Ranges: bytes`.

#### 2.4.4 Reading Progress & Bookmarks Endpoints
* **`GET /api/books/:id/progress`**
  * **Response:**
    ```json
    {
      "book_id": 1,
      "current_page": 45,
      "scroll_anchor": "p_045",
      "is_finished": false,
      "last_read_at": "2026-11-06T03:30:00Z"
    }
    ```
* **`POST /api/books/:id/progress`**
  * **Request Body:**
    ```json
    { "page": 46, "scroll_anchor": "p_046", "is_finished": false }
    ```
  * **Response:** `200 OK`
* **`GET /api/books/:id/bookmarks`**
  * **Response:** Array of bookmarks for the book.
* **`POST /api/books/:id/bookmarks`**
  * **Request Body:** `{ "page_number": 46, "title": "Interesting quote", "xpath": "/html/body/p[12]" }`
  * **Response:** `201 Created`
* **`DELETE /api/bookmarks/:id`**
  * **Response:** `204 No Content`

---

### 2.5 Embedded Web Reader UI Specifications

The Web UI is delivered directly from the Axum server as a single lightweight, high-performance HTML/CSS/JS application served at `GET /`. It uses modern, zero-build tools (HTMX + Alpine.js + Vanilla JS) styled with clean modern CSS (dark mode by default).

#### 1. Library Grid View (`/`)
* **Top Navigation Bar:**
  * Logo & App Title: `BookCore`.
  * Quick search input (instant title/series search filter).
  * Format Filter Pills: `All`, `EPUB`, `CBZ / Manga`, `PDF`.
  * "Scan Library" button (triggers `POST /api/scan` with spinning indicator).
  * "Stats & Streak" button (opens the Analytics Dashboard modal).
* **Book Cards Grid:**
  * Displays book cover thumbnail, format badge (e.g. green for EPUB, blue for CBZ, red for PDF), Title, Series/Volume name.
  * Visual progress bar at the bottom of the card (`current_page / page_count`).
  * "Continue Reading" click action launches the appropriate reader mode.

#### 2. Manga / CBZ Reader Mode
* Full-screen distraction-free reader interface.
* **Navigation:** Left/Right arrow keys, A/D keys, or tapping screen sides (supports Right-to-Left Manga mode or Left-to-Right Comic mode).
* **Fit Mode Toggle:** `Fit to Height`, `Fit to Width`, or `Original`.
* **Page Scrubber:** Slider at bottom displaying `Page X / Y` with instant scrubbing.
* Auto-syncs progress via `POST /api/books/:id/progress` on page turn.

#### 3. EPUB Reader Mode
* Clean typography reader with customizable reading settings drawer:
  * Font size slider (12px to 28px).
  * Color Themes: Dark (OLED black/charcoal), Sepia (warm paper), Light (high contrast).
  * Font Family Selector: Sans-Serif, Serif, Monospace.
* **Chapter Selector:** Sidebar drawer displaying Table of Contents (`GET /api/books/:id/chapters`).
* Seamless Previous Chapter / Next Chapter navigation buttons.

#### 4. PDF Reader Mode
* Embedded native high-performance PDF viewer with standard zoom, rotation, and continuous scroll controls.
* Floating page tracker bar transmitting page viewing state to BookCore reading progress.

#### 5. Bookmarks Drawer / Panel
* Slide-out panel accessible via a bookmark icon or keyboard shortcut `B`.
* Shows list of all bookmarks in current book with page number, timestamp, and preview title.
* One-click jump to bookmarked page or delete bookmark.

---

## 3. Technical Specification: Reading Timer & Tracker Module

The Reading Timer & Tracker is a core differentiator for BookCore, providing automatic, verified reading habit tracking, Pomodoro focus cycles, pages-per-minute (PPM) telemetry, and reading streaks.

### 3.1 Data Model & Session Lifecycle

#### Reading Sessions Table (`reading_sessions`)
| Column | Type | Description |
|---|---|---|
| `id` | `INTEGER PRIMARY KEY` | Auto-incrementing session identifier |
| `book_id` | `INTEGER` | Foreign key referencing `books(id)` |
| `started_at` | `DATETIME` | Timestamp when user opened reader / started timer |
| `last_heartbeat_at` | `DATETIME` | Timestamp of latest verified heartbeat from client |
| `ended_at` | `DATETIME` | Final timestamp when session was stopped or timed out |
| `active_seconds` | `INTEGER` | Total accumulated active reading seconds (excluding pauses/idle) |
| `start_page` | `INTEGER` | Book page when session began |
| `end_page` | `INTEGER` | Latest book page reached in session |
| `pages_read` | `INTEGER` | `max(0, end_page - start_page)` |
| `is_pomodoro` | `BOOLEAN` | Whether this session is tied to a Pomodoro focus interval |
| `goal_minutes` | `INTEGER` | Configured target duration (e.g., 25 for Pomodoro, 30 for daily goal) |
| `is_active` | `BOOLEAN` | 1 if active, 0 if completed or closed |

#### Daily Stats Rollup Table (`daily_stats`)
| Column | Type | Description |
|---|---|---|
| `date_str` | `TEXT PRIMARY KEY` | Calendar date formatted as `YYYY-MM-DD` |
| `total_seconds` | `INTEGER` | Total seconds read on this day |
| `pages_read` | `INTEGER` | Total pages turned across all books on this day |
| `books_touched` | `INTEGER` | Count of distinct books read on this day |
| `sessions_count` | `INTEGER` | Number of reading sessions recorded |

---

### 3.2 Client-Side Activity & Idle Engine

To guarantee accurate telemetry without inflating reading time when users walk away or minimize tabs, client-side JavaScript (`static/app.js`) implements a strict multi-stage activity detector:

1. **Tab Visibility (`document.visibilitychange`):**
   * If `document.hidden` becomes true (user switches tabs, minimizes window, or locks screen):
     * The client immediately triggers `POST /api/timer/pause`.
     * The heartbeat timer is suspended.
   * When the tab becomes visible again:
     * The client triggers `POST /api/timer/heartbeat` with resume state.
2. **Window Focus / Blur:**
   * `window.addEventListener('blur')` and `window.addEventListener('focus')` reinforce the visibility state.
3. **Idle Detection (60-second inactivity timeout):**
   * User interaction listeners: `mousemove`, `keydown`, `touchstart`, `scroll`.
   * Any interaction resets an internal client-side countdown timer to 60 seconds.
   * If no interaction occurs for 60 consecutive seconds:
     * The reader status changes to `PAUSED (IDLE)`.
     * An unobtrusive overlay appears: *"Reading paused due to inactivity. Click or press any key to resume."*
     * The client sends `POST /api/timer/pause`.
4. **Heartbeat Loop (15-second tick):**
   * While active and not idle, the client sends a heartbeat every 15 seconds:
     ```json
     POST /api/timer/heartbeat
     {
       "session_id": 14,
       "current_page": 52,
       "delta_seconds": 15
     }
     ```

---

### 3.3 Timer REST Endpoints

* **`POST /api/timer/start`**
  * **Request:**
    ```json
    {
      "book_id": 1,
      "current_page": 45,
      "is_pomodoro": true,
      "goal_minutes": 25
    }
    ```
  * **Behavior:** Closes any previously dangling active sessions for this client, creates a new row in `reading_sessions`, and returns session details.
  * **Response:** `200 OK`
    ```json
    { "session_id": 14, "started_at": "2026-11-06T03:30:00Z", "goal_minutes": 25 }
    ```

* **`POST /api/timer/heartbeat`**
  * **Request:**
    ```json
    {
      "session_id": 14,
      "current_page": 52,
      "delta_seconds": 15
    }
    ```
  * **Behavior:**
    * Verifies `session_id` is active.
    * Updates `last_heartbeat_at = CURRENT_TIMESTAMP`.
    * Increments `active_seconds = active_seconds + delta_seconds`.
    * Updates `end_page = current_page` and `pages_read = max(pages_read, current_page - start_page)`.
    * Atomically updates `daily_stats` for today's date (`date('now')`).
  * **Response:** `200 OK`
    ```json
    { "status": "active", "active_seconds": 450, "pages_read": 7 }
    ```

* **`POST /api/timer/pause`**
  * **Request:** `{ "session_id": 14 }`
  * **Behavior:** Records latest heartbeat, marks state as paused, prevents further time accumulation until resumed.

* **`POST /api/timer/stop`**
  * **Request:** `{ "session_id": 14, "final_page": 52 }`
  * **Behavior:** Sets `is_active = 0`, `ended_at = CURRENT_TIMESTAMP`, computes final `pages_read` and stats rollup.

* **`GET /api/timer/stats`**
  * **Query Params:** `book_id` (optional, for book-specific stats).
  * **Response:** `200 OK`
    ```json
    {
      "current_streak_days": 12,
      "longest_streak_days": 28,
      "total_reading_seconds": 43200,
      "total_reading_hours": 12.0,
      "total_pages_read": 580,
      "pages_per_minute": 0.81,
      "today": {
        "seconds": 2700,
        "minutes": 45,
        "pages": 38,
        "goal_minutes": 30,
        "goal_achieved": true
      },
      "recent_days": [
        { "date": "2026-11-05", "minutes": 42, "pages": 35 },
        { "date": "2026-11-06", "minutes": 45, "pages": 38 }
      ]
    }
    ```

---

### 3.4 Pomodoro Mode & Goal Countdown Ring

* **Top Bar Reader Widget:**
  * Fixed to the reader header bar: an SVG circular progress ring displaying either elapsed time or countdown time.
  * **Pomodoro Option (25 min focus / 5 min break):**
    * Ring fills dynamically from 0% to 100% as the 25 minutes elapse.
    * When 25 minutes are reached:
      * A gentle audio chime plays (Web Audio API synthesis, zero external mp3 files).
      * A toast notification alerts: *"Great focus session! Time for a 5-minute break."*
  * **Daily Reading Goal Mode:**
    * Displays daily goal progress (e.g. `25 / 30 mins`).
    * Progress ring turns gold/green upon achieving the daily goal.

---

### 3.5 Daily Streak & PPM (Pages-Per-Minute) Calculation Engine

#### 1. Daily Streak Algorithm
1. Query distinct active reading dates from `daily_stats` where `total_seconds >= 60` (or `pages_read >= 1`), ordered descending by `date_str`:
   ```sql
   SELECT date_str FROM daily_stats 
   WHERE total_seconds >= 60 OR pages_read >= 1 
   ORDER BY date_str DESC;
   ```
2. Let $D_0$ be today's date in local time (`YYYY-MM-DD`).
3. Check if the latest recorded date is today ($D_0$) or yesterday ($D_0 - 1$).
   * If neither, current streak = 0.
   * If today or yesterday, iterate backwards day-by-day:
     * While $D_{k-1} = D_k - 1\text{ day}$, streak count increments by 1.
     * The first missing consecutive calendar day breaks the streak.
4. Longest streak is computed by scanning all continuous runs of consecutive dates across the entire table history.

#### 2. Pages-Per-Minute (PPM) Formula
* **Session PPM:**
  $$\text{PPM} = \frac{\text{pages\_read}}{\frac{\text{active\_seconds}}{60.0}}$$
* **Rolling Global PPM:**
  $$\text{Global PPM} = \frac{\sum \text{pages\_read}}{\sum \frac{\text{active\_seconds}}{60.0}}$$
* Guarded against division by zero (if `active_seconds < 60`, PPM reports 0.0 or pending until 1 full minute of verified activity is accumulated).

---

### 3.6 Analytics & Stats Dashboard Modal Spec

When the user clicks the "Stats & Streak" button in the library header, a modal opens displaying:
1. **Streak Counter Hero Banner:**
   * Large flame icon with `"12 Day Streak!"` and encouraging message.
   * Total days read all-time, longest streak all-time.
2. **Speed & Efficiency Card:**
   * Current Average Pace: e.g. `0.85 Pages / Minute` (~70 seconds per page).
   * Estimated time to finish currently active books based on remaining pages $\times$ personal PPM.
3. **Weekly Activity Heatmap / Bar Chart:**
   * 7-day visual bar chart of minutes read per day.
   * Visual checkmarks for days reaching the daily goal.
4. **Recent Sessions History Table:**
   * Shows last 10 reading sessions with Book Title, Date, Active Duration, Pages Read, and Session PPM.

---

## 4. Operational & Deployment Blueprint (Port 3435)

### 4.1 Configuration Management (`src/config.rs`)
* Config parameters read from environment variables with safe defaults:
  * `PORT`: Server listen port (default: `3435`).
  * `BOOKCORE_DB_PATH`: Path to SQLite database file (default: `/data/bookcore.db`).
  * `BOOKCORE_BOOKS_DIR`: Root directory for auto-scanning books (default: `/data/books`).
  * `BOOKCORE_CACHE_DIR`: Directory for extracted covers and temporary files (default: `/data/cache`).

### 4.2 Production Dockerfile (`/home/mustafa/agy/bookcore/Dockerfile`)
A lightweight, multi-stage build using `rust:alpine` or `debian:bookworm-slim` for pure-Rust compilation and minimal image size (<30MB).

```dockerfile
# Stage 1: Chef & Builder
FROM rust:1.82-bookworm AS builder
WORKDIR /app

# Copy dependency manifests
COPY Cargo.toml ./

# Create dummy source to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Copy real source code and static web assets
COPY src ./src

# Build release binary
RUN touch src/main.rs && cargo build --release

# Stage 2: Runtime Minimal Image
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates tzdata && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/bookcore /usr/local/bin/bookcore

# Expose hardcoded default port 3435
EXPOSE 3435

ENV PORT=3435
ENV BOOKCORE_DB_PATH=/data/bookcore.db
ENV BOOKCORE_BOOKS_DIR=/data/books
ENV BOOKCORE_CACHE_DIR=/data/cache

VOLUME ["/data"]

CMD ["/usr/local/bin/bookcore"]
```

### 4.3 Docker Compose (`/home/mustafa/agy/bookcore/docker-compose.yml`)
```yaml
version: '3.8'

services:
  bookcore:
    build:
      context: .
      dockerfile: Dockerfile
    container_name: bookcore
    restart: unless-stopped
    ports:
      - "3435:3435"
    environment:
      - PORT=3435
      - BOOKCORE_DB_PATH=/data/bookcore.db
      - BOOKCORE_BOOKS_DIR=/data/books
      - BOOKCORE_CACHE_DIR=/data/cache
      - RUST_LOG=info,bookcore=debug
    volumes:
      - ./data:/data
```

---

## 5. Verification & Implementation Roadmap

| Phase | Milestone | Deliverable |
|---|---|---|
| **Phase 1** | Scaffolding & DB | `Cargo.toml`, SQLite connection pool, DDL migrations, parent directory auto-creation |
| **Phase 2** | Scanner & Parsers | `walkdir` recursive scanning, CBZ `ComicInfo.xml` + regex parsing, pure-Rust EPUB OPF/spine reader |
| **Phase 3** | Reader APIs | Page image streaming, EPUB chapter scoping & asset delivery, PDF byte-range streaming |
| **Phase 4** | Timer & Tracker | Heartbeat protocol, idle detection, Pomodoro mode, streak & PPM calculation |
| **Phase 5** | Web Reader UI | Embedded single-page reader (HTMX + Alpine.js + CSS) on port `3435` |

All specifications in this document are self-contained and ready for immediate implementation by subsequent subagents.
