pub const PRAGMAS: &[&str] = &[
    "PRAGMA journal_mode = WAL;",
    "PRAGMA synchronous = NORMAL;",
    "PRAGMA foreign_keys = ON;",
    "PRAGMA busy_timeout = 5000;",
];

pub const SCHEMA_DDL: &[&str] = &[
    r#"
    CREATE TABLE IF NOT EXISTS libraries (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL UNIQUE,
        path TEXT NOT NULL,
        created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
        updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
    );
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS books (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        library_id INTEGER NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
        title TEXT NOT NULL,
        series TEXT NOT NULL DEFAULT '',
        volume TEXT NOT NULL DEFAULT '',
        chapter TEXT NOT NULL DEFAULT '',
        format TEXT NOT NULL,
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
    "#,
    "CREATE INDEX IF NOT EXISTS idx_books_library_id ON books(library_id);",
    "CREATE INDEX IF NOT EXISTS idx_books_format ON books(format);",
    "CREATE INDEX IF NOT EXISTS idx_books_series ON books(series);",
    r#"
    CREATE TABLE IF NOT EXISTS book_chapters (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        book_id INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
        chapter_index INTEGER NOT NULL,
        title TEXT NOT NULL,
        content_href TEXT NOT NULL,
        word_count INTEGER NOT NULL DEFAULT 0,
        UNIQUE(book_id, chapter_index)
    );
    "#,
    "CREATE INDEX IF NOT EXISTS idx_book_chapters_book_id ON book_chapters(book_id);",
    r#"
    CREATE TABLE IF NOT EXISTS reading_progress (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        book_id INTEGER NOT NULL UNIQUE REFERENCES books(id) ON DELETE CASCADE,
        current_page INTEGER NOT NULL DEFAULT 0,
        scroll_anchor TEXT,
        is_finished BOOLEAN NOT NULL DEFAULT 0,
        total_reads INTEGER NOT NULL DEFAULT 0,
        last_read_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
    );
    "#,
    "CREATE INDEX IF NOT EXISTS idx_reading_progress_book ON reading_progress(book_id);",
    r#"
    CREATE TABLE IF NOT EXISTS bookmarks (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        book_id INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
        page_number INTEGER NOT NULL,
        title TEXT,
        xpath TEXT,
        created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
    );
    "#,
    "CREATE INDEX IF NOT EXISTS idx_bookmarks_book ON bookmarks(book_id);",
    r#"
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
    "#,
    "CREATE INDEX IF NOT EXISTS idx_sessions_book ON reading_sessions(book_id);",
    "CREATE INDEX IF NOT EXISTS idx_sessions_active ON reading_sessions(is_active);",
    "CREATE INDEX IF NOT EXISTS idx_sessions_started ON reading_sessions(started_at);",
    r#"
    CREATE TABLE IF NOT EXISTS daily_stats (
        date_str TEXT PRIMARY KEY,
        total_seconds INTEGER NOT NULL DEFAULT 0,
        pages_read INTEGER NOT NULL DEFAULT 0,
        books_touched INTEGER NOT NULL DEFAULT 0,
        sessions_count INTEGER NOT NULL DEFAULT 0
    );
    "#,
];
