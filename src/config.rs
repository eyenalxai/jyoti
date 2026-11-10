use std::env;
use std::path::PathBuf;
use std::time::Duration;

use directories::ProjectDirs;

const DEFAULT_MIIO_TIMEOUT: Duration = Duration::from_secs(4);
const MAX_MIIO_TIMEOUT: Duration = Duration::from_secs(60);

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
        let miio_timeout = timeout_from(env::var("JYOTI_MIIO_TIMEOUT").ok().as_deref());

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

fn timeout_from(value: Option<&str>) -> Duration {
    value
        .and_then(parse_duration)
        .map_or(DEFAULT_MIIO_TIMEOUT, |duration| {
            duration.min(MAX_MIIO_TIMEOUT)
        })
}

fn parse_duration(value: &str) -> Option<Duration> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    let duration = if let Some(number) = value.strip_suffix("ms") {
        Duration::from_millis(number.trim().parse::<u64>().ok()?)
    } else if let Some(number) = value
        .strip_suffix(" seconds")
        .or_else(|| value.strip_suffix('s'))
    {
        seconds(number)?
    } else {
        seconds(value)?
    };
    Some(duration)
}

fn seconds(value: &str) -> Option<Duration> {
    let total = value.trim().parse::<f64>().ok()?;
    Duration::try_from_secs_f64(total).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_timeouts_parse() {
        assert_eq!(parse_duration("4s"), Some(DEFAULT_MIIO_TIMEOUT));
        assert_eq!(parse_duration("4 seconds"), Some(DEFAULT_MIIO_TIMEOUT));
        assert_eq!(parse_duration("4"), Some(DEFAULT_MIIO_TIMEOUT));
        assert_eq!(parse_duration("400ms"), Some(Duration::from_millis(400)));
        assert_eq!(parse_duration("0"), Some(Duration::ZERO));
    }

    #[test]
    fn malformed_timeouts_fall_back_without_panicking() {
        for value in [
            "-1", "-1s", "-400ms", "NaN", "inf", "-inf", "1e999", "1e300",
        ] {
            assert_eq!(timeout_from(Some(value)), DEFAULT_MIIO_TIMEOUT, "{value}");
        }
        assert_eq!(timeout_from(None), DEFAULT_MIIO_TIMEOUT);
    }

    #[test]
    fn the_parser_does_not_apply_the_timeout_cap() {
        assert_eq!(parse_duration("120s"), Some(Duration::from_secs(120)));
    }

    #[test]
    fn long_timeouts_are_capped() {
        assert_eq!(timeout_from(Some("120s")), MAX_MIIO_TIMEOUT);
        assert_eq!(timeout_from(Some("999999999999ms")), MAX_MIIO_TIMEOUT);
    }
}
