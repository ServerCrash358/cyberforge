//! Settings read from the environment once at startup, with the same names as the Python API.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;

const DEFAULT_PORT: u16 = 8000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub db_path: PathBuf,
    pub hmac_key: Option<String>,
    pub license_public_key: Option<PathBuf>,
    pub addr: SocketAddr,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConfigError {
    BadPort(String),
    BadHost(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadPort(value) => write!(f, "SCAM_MAP_PORT is not a port number: {value}"),
            Self::BadHost(value) => write!(f, "SCAM_MAP_HOST is not an IP address: {value}"),
        }
    }
}

impl std::error::Error for ConfigError {}

impl Config {
    /// Reads `SCAM_MAP_DB_PATH`, `SCAM_MAP_HMAC_KEY`, `SCAM_MAP_LICENSE_PUBLIC_KEY`,
    /// `SCAM_MAP_HOST` and `SCAM_MAP_PORT`.
    ///
    /// # Errors
    /// A host or port that does not parse.
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    /// Same as [`Config::from_env`] with any source of variables, so tests need no real environment.
    ///
    /// # Errors
    /// A host or port that does not parse.
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let port = match get("SCAM_MAP_PORT") {
            Some(value) => value.parse().map_err(|_| ConfigError::BadPort(value))?,
            None => DEFAULT_PORT,
        };
        let host = match get("SCAM_MAP_HOST") {
            Some(value) => value.parse().map_err(|_| ConfigError::BadHost(value))?,
            None => IpAddr::V4(Ipv4Addr::LOCALHOST),
        };
        Ok(Self {
            db_path: get("SCAM_MAP_DB_PATH")
                .map_or_else(|| "scam_map.sqlite".into(), PathBuf::from),
            hmac_key: get("SCAM_MAP_HMAC_KEY"),
            license_public_key: get("SCAM_MAP_LICENSE_PUBLIC_KEY").map(PathBuf::from),
            addr: SocketAddr::new(host, port),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn config(vars: &[(&str, &str)]) -> Result<Config, ConfigError> {
        let vars: HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        Config::from_lookup(|name| vars.get(name).cloned())
    }

    #[test]
    fn defaults_match_the_python_api() {
        let c = config(&[]).unwrap();
        assert_eq!(c.db_path, PathBuf::from("scam_map.sqlite"));
        assert_eq!(c.addr, "127.0.0.1:8000".parse().unwrap());
        assert_eq!(c.hmac_key, None);
    }

    #[test]
    fn reads_every_variable() {
        let c = config(&[
            ("SCAM_MAP_DB_PATH", "data/x.sqlite"),
            ("SCAM_MAP_HMAC_KEY", "k"),
            ("SCAM_MAP_LICENSE_PUBLIC_KEY", "keys/pub.pem"),
            ("SCAM_MAP_HOST", "0.0.0.0"),
            ("SCAM_MAP_PORT", "9000"),
        ])
        .unwrap();
        assert_eq!(c.db_path, PathBuf::from("data/x.sqlite"));
        assert_eq!(c.hmac_key.as_deref(), Some("k"));
        assert_eq!(c.license_public_key, Some(PathBuf::from("keys/pub.pem")));
        assert_eq!(c.addr, "0.0.0.0:9000".parse().unwrap());
    }

    #[test]
    fn refuses_a_bad_port_or_host() {
        assert_eq!(
            config(&[("SCAM_MAP_PORT", "http")]),
            Err(ConfigError::BadPort("http".into()))
        );
        assert_eq!(
            config(&[("SCAM_MAP_HOST", "laptop")]),
            Err(ConfigError::BadHost("laptop".into()))
        );
    }
}
