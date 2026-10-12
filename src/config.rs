//! Settings read from environment variables at startup.

use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    /// SQLite connection string, e.g. `sqlite:data/cissp.db`.
    pub database_url: String,
    /// When false, only the very first account can be created.
    pub allow_signups: bool,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:data/cissp.db".to_string()),
            allow_signups: env::var("ALLOW_SIGNUPS").is_ok_and(|v| parse_bool(&v)),
        }
    }
}

fn parse_bool(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

#[cfg(test)]
mod tests {
    use super::parse_bool;

    #[test]
    fn parses_truthy_values() {
        for v in ["1", "true", "TRUE", " yes ", "on"] {
            assert!(parse_bool(v), "{v:?} should be true");
        }
    }

    #[test]
    fn everything_else_is_false() {
        for v in ["", "0", "false", "no", "off", "maybe"] {
            assert!(!parse_bool(v), "{v:?} should be false");
        }
    }
}
