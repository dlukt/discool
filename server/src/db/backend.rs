#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseBackend {
    Postgres,
    Sqlite,
}

impl DatabaseBackend {
    pub fn detect(url: &str) -> Result<Self, String> {
        Self::from_url(url)
    }

    pub fn from_url(url: &str) -> Result<Self, String> {
        if url.starts_with("postgres://") || url.starts_with("postgresql://") {
            Ok(Self::Postgres)
        } else if url.starts_with("sqlite://") || url.starts_with("sqlite:") {
            Ok(Self::Sqlite)
        } else {
            // Do not include the full URL in the error (it may contain credentials).
            let scheme = url.split_once(':').map(|(s, _)| s).unwrap_or("<unknown>");
            Err(format!("unsupported database URL scheme: {scheme}"))
        }
    }

    pub fn returning_clause(&self) -> &'static str {
        match self {
            Self::Postgres => "RETURNING",
            Self::Sqlite => "RETURNING",
        }
    }

    pub fn upsert_syntax(&self) -> &'static str {
        match self {
            Self::Postgres => "ON CONFLICT",
            Self::Sqlite => "ON CONFLICT",
        }
    }

    pub fn now_function(&self) -> &'static str {
        match self {
            Self::Postgres => "NOW()",
            Self::Sqlite => "datetime('now')",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_url_detects_supported_backends() {
        assert_eq!(
            DatabaseBackend::from_url("postgres://localhost/db").unwrap(),
            DatabaseBackend::Postgres
        );
        assert_eq!(
            DatabaseBackend::from_url("postgresql://localhost/db").unwrap(),
            DatabaseBackend::Postgres
        );
        assert_eq!(
            DatabaseBackend::from_url("sqlite::memory:").unwrap(),
            DatabaseBackend::Sqlite
        );
        assert_eq!(
            DatabaseBackend::from_url("sqlite://./data/discool.db").unwrap(),
            DatabaseBackend::Sqlite
        );
    }

    #[test]
    fn from_url_rejects_unknown_scheme() {
        let err = DatabaseBackend::from_url("mysql://localhost/db").unwrap_err();
        assert!(err.contains("unsupported"), "unexpected error: {err}");
        assert!(
            !err.contains("mysql://localhost/db"),
            "error should not include full URL: {err}"
        );
    }

    #[test]
    fn detect_detects_supported_backends() {
        assert_eq!(
            DatabaseBackend::detect("postgres://localhost/db").unwrap(),
            DatabaseBackend::Postgres
        );
        assert_eq!(
            DatabaseBackend::detect("sqlite::memory:").unwrap(),
            DatabaseBackend::Sqlite
        );
    }
}
