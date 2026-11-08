use std::env;
use std::path::PathBuf;
use std::time::Duration;

use directories::ProjectDirs;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub config_path: PathBuf,
    pub preferences_path: PathBuf,
    pub keyring_service: String,
    pub miio_port: u16,
    pub miio_timeout: Duration,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let config_path = env::var_os("JYOTI_CONFIG_PATH")
            .filter(|value| !value.is_empty())
            .map_or_else(default_config_path, PathBuf::from);
        let preferences_path = config_path.parent().map_or_else(
            || PathBuf::from("prefs.json"),
            |parent| parent.join("prefs.json"),
        );
        let keyring_service = env::var("JYOTI_KEYRING_SERVICE")
            .ok()
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "jyoti".to_owned());
        let miio_port = env::var("JYOTI_MIIO_PORT")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(54_321);
        let miio_timeout = env::var("JYOTI_MIIO_TIMEOUT")
            .ok()
            .and_then(|value| parse_duration(&value))
            .unwrap_or_else(|| Duration::from_secs(4));

        Self {
            config_path,
            preferences_path,
            keyring_service,
            miio_port,
            miio_timeout,
        }
    }
}

fn default_config_path() -> PathBuf {
    ProjectDirs::from("", "", "jyoti").map_or_else(
        || {
            let home = env::var_os("HOME").map_or_else(PathBuf::new, PathBuf::from);
            home.join(".config/jyoti/bulbs.json")
        },
        |dirs| dirs.config_dir().join("bulbs.json"),
    )
}

fn parse_duration(value: &str) -> Option<Duration> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    if let Some(number) = value.strip_suffix("ms") {
        return number.trim().parse::<u64>().ok().map(Duration::from_millis);
    }
    if let Some(number) = value.strip_suffix('s') {
        return number
            .trim()
            .parse::<f64>()
            .ok()
            .map(Duration::from_secs_f64);
    }
    if let Some(number) = value.strip_suffix(" seconds") {
        return number
            .trim()
            .parse::<f64>()
            .ok()
            .map(Duration::from_secs_f64);
    }
    value.parse::<f64>().ok().map(Duration::from_secs_f64)
}
