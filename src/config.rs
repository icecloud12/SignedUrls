use std::{env, fmt::Display, net::IpAddr, net::SocketAddr, path::PathBuf, str::FromStr};

/// Application settings, read once from the environment at startup.
#[derive(Debug, Clone)]
pub struct Config {
    pub address: SocketAddr,
    pub prefix: String,
    /// Public host (and port) used when building signed URLs, e.g. `localhost:8080`.
    pub replaced_url: String,
    pub database_uri: String,
    pub database_name: String,
    pub default_duration_secs: u64,
    /// Root folder for uploaded files. Stored file records keep paths under it, so don't change it on a live deployment.
    pub data_dir: PathBuf,
    /// Certificate files to serve. When `None`, a self-signed certificate is generated at startup.
    pub tls_files: Option<TlsFiles>,
}

#[derive(Debug, Clone)]
pub struct TlsFiles {
    pub cert: PathBuf,
    pub key: PathBuf,
}

impl Config {
    /// Reads every variable before failing, so all problems are reported at once.
    pub fn from_env() -> Result<Self, String> {
        let mut env = EnvReader::default();
        let ip = env.parse::<IpAddr>("ADDRESS");
        let port = env.parse::<u16>("PORT");
        let prefix = env.parse::<String>("PREFIX");
        let replaced_url = env.parse::<String>("REPLACED_URL");
        let database_uri = env.parse::<String>("DATABASE_URI");
        let database_name = env.parse::<String>("DATABASE_NAME");
        let default_duration_secs = env.parse::<u64>("DEFAULT_DURATION_AS_SECONDS");
        let data_dir = env.parse::<PathBuf>("DATA_DIR");
        let tls_files = match (
            env.optional::<PathBuf>("TLS_CERT_PATH"),
            env.optional::<PathBuf>("TLS_KEY_PATH"),
        ) {
            (Some(cert), Some(key)) => Some(TlsFiles { cert, key }),
            (None, None) => None,
            _ => {
                env.errors
                    .push("TLS_CERT_PATH and TLS_KEY_PATH must be set together".to_string());
                None
            }
        };

        let build = || {
            Some(Config {
                address: SocketAddr::new(ip?, port?),
                prefix: prefix?,
                replaced_url: replaced_url?,
                database_uri: database_uri?,
                database_name: database_name?,
                default_duration_secs: default_duration_secs?,
                data_dir: data_dir?,
                tls_files,
            })
        };
        match build() {
            Some(config) if env.errors.is_empty() => Ok(config),
            _ => Err(env.errors.join("\n")),
        }
    }
}

#[derive(Default)]
struct EnvReader {
    errors: Vec<String>,
}

impl EnvReader {
    fn parse<T>(&mut self, key: &str) -> Option<T>
    where
        T: FromStr,
        T::Err: Display,
    {
        if env::var_os(key).is_none() {
            self.errors.push(format!("{key} is not set"));
            return None;
        }
        self.optional(key)
    }

    /// Like `parse`, but a missing variable is not an error.
    fn optional<T>(&mut self, key: &str) -> Option<T>
    where
        T: FromStr,
        T::Err: Display,
    {
        let raw = env::var(key).ok()?;
        match raw.parse() {
            Ok(value) => Some(value),
            Err(e) => {
                self.errors.push(format!("{key}={raw:?} is invalid: {e}"));
                None
            }
        }
    }
}
