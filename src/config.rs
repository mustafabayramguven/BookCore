use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub port: u16,
    pub db_path: PathBuf,
    pub books_dir: PathBuf,
    pub cache_dir: PathBuf,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let port = std::env::var("PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(3435);

        let db_path = std::env::var("BOOKCORE_DB_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/data/bookcore.db"));

        let books_dir = std::env::var("BOOKCORE_BOOKS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/books"));

        let cache_dir = std::env::var("BOOKCORE_CACHE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/data/cache"));

        Self {
            port,
            db_path,
            books_dir,
            cache_dir,
        }
    }

    pub fn ensure_dirs(&self) -> std::io::Result<()> {
        if let Some(parent) = self.db_path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                let _ = std::fs::create_dir_all(parent);
            }
        }
        if !self.books_dir.exists() {
            let _ = std::fs::create_dir_all(&self.books_dir);
        }
        if !self.cache_dir.exists() {
            let _ = std::fs::create_dir_all(&self.cache_dir);
        }
        let covers_dir = self.cache_dir.join("covers");
        if !covers_dir.exists() {
            let _ = std::fs::create_dir_all(covers_dir);
        }
        Ok(())
    }

    pub fn covers_dir(&self) -> PathBuf {
        self.cache_dir.join("covers")
    }

    pub fn db_url(&self) -> String {
        format!("sqlite://{}", self.db_path.to_string_lossy())
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.port, 3435);
        assert!(cfg.db_url().starts_with("sqlite://"));
    }
}
