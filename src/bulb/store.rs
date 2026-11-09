use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;

use crate::bulb::model::{Bulb, BulbFile, BulbListIssue, validate_bulbs};
use crate::error::StoreError;
use crate::screen::gain::{SYNC_GAIN_DEFAULT, normalize_sync_gain};
use crate::screen::intensity::{SYNC_INTENSITY_DEFAULT, normalize_sync_intensity};

#[derive(Clone, Debug)]
pub struct BulbStore {
    path: PathBuf,
}

impl BulbStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn load(&self) -> Result<Vec<Bulb>, StoreError> {
        let text = read_text(&self.path)?;
        let Some(text) = text else {
            return Ok(Vec::new());
        };

        let file: BulbFile = serde_json::from_str(&text).map_err(|source| StoreError::Decode {
            path: self.path.clone(),
            source,
        })?;

        if file.version != BulbFile::VERSION {
            return Err(StoreError::Invalid {
                path: self.path.clone(),
                message: format!(
                    "{} uses bulb file version {}, but this app reads version {}.",
                    self.path.display(),
                    file.version,
                    BulbFile::VERSION
                ),
            });
        }

        validate_bulbs(&file.bulbs).map_err(|issue| store_issue(&self.path, issue))?;

        Ok(file.bulbs)
    }

    pub fn save(&self, bulbs: &[Bulb]) -> Result<(), StoreError> {
        let file = BulbFile {
            version: BulbFile::VERSION,
            bulbs: bulbs.to_vec(),
        };
        let text = serde_json::to_string_pretty(&file).map_err(|source| StoreError::Decode {
            path: self.path.clone(),
            source,
        })?;
        write_text(&self.path, &format!("{text}\n"))
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Appearance {
    #[default]
    System,
    Light,
    Dark,
}

impl Appearance {
    pub fn dark(self, system_dark: bool) -> bool {
        match self {
            Appearance::System => system_dark,
            Appearance::Light => false,
            Appearance::Dark => true,
        }
    }

    pub fn index(self) -> usize {
        match self {
            Appearance::Light => 0,
            Appearance::Dark => 1,
            Appearance::System => 2,
        }
    }

    pub fn from_index(index: usize) -> Self {
        match index {
            0 => Appearance::Light,
            1 => Appearance::Dark,
            _ => Appearance::System,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Preferences {
    pub sync_gain: f32,
    pub colour_intensity: f32,
    pub appearance: Appearance,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            sync_gain: SYNC_GAIN_DEFAULT,
            colour_intensity: SYNC_INTENSITY_DEFAULT,
            appearance: Appearance::System,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PreferencesStore {
    path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
struct PreferencesFile {
    version: u8,
    #[serde(rename = "syncGain")]
    sync_gain: f32,
    #[serde(rename = "colourIntensity", default)]
    colour_intensity: f32,
    #[serde(default)]
    appearance: Appearance,
}

impl PreferencesStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn load(&self) -> Preferences {
        let Ok(Some(text)) = read_text(&self.path) else {
            return Preferences::default();
        };
        let Ok(file) = serde_json::from_str::<PreferencesFile>(&text) else {
            return Preferences::default();
        };
        if file.version != 1 {
            return Preferences::default();
        }
        Preferences {
            sync_gain: normalize_sync_gain(file.sync_gain),
            colour_intensity: normalize_sync_intensity(file.colour_intensity),
            appearance: file.appearance,
        }
    }

    pub fn save(&self, preferences: Preferences) -> Result<(), StoreError> {
        let file = PreferencesFile {
            version: 1,
            sync_gain: normalize_sync_gain(preferences.sync_gain),
            colour_intensity: normalize_sync_intensity(preferences.colour_intensity),
            appearance: preferences.appearance,
        };
        let text = serde_json::to_string_pretty(&file).map_err(|source| StoreError::Decode {
            path: self.path.clone(),
            source,
        })?;
        write_text(&self.path, &format!("{text}\n"))
    }
}

fn store_issue(path: &Path, issue: BulbListIssue) -> StoreError {
    match issue {
        BulbListIssue::DuplicateId => StoreError::DuplicateId {
            path: path.to_path_buf(),
        },
        issue => StoreError::Invalid {
            path: path.to_path_buf(),
            message: format!("{} {issue}", path.display()),
        },
    }
}

fn read_text(path: &Path) -> Result<Option<String>, StoreError> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(StoreError::Read {
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn write_text(path: &Path, text: &str) -> Result<(), StoreError> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|source| StoreError::Write {
        path: path.to_path_buf(),
        source,
    })?;

    let mut temporary = NamedTempFile::new_in(parent).map_err(|source| StoreError::Write {
        path: path.to_path_buf(),
        source,
    })?;
    temporary
        .write_all(text.as_bytes())
        .map_err(|source| StoreError::Write {
            path: path.to_path_buf(),
            source,
        })?;
    temporary.flush().map_err(|source| StoreError::Write {
        path: path.to_path_buf(),
        source,
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(temporary.path(), fs::Permissions::from_mode(0o600)).map_err(
            |source| StoreError::Write {
                path: path.to_path_buf(),
                source,
            },
        )?;
    }

    temporary.persist(path).map_err(|error| StoreError::Write {
        path: path.to_path_buf(),
        source: error.error,
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bulb::model::Bulb;

    #[test]
    fn bulbs_round_trip_through_the_atomic_store() -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("bulbs.json");
        let store = BulbStore::new(path.clone());
        let bulb = Bulb {
            id: "00000000-0000-4000-8000-000000000001".to_owned(),
            name: "Desk lamp".to_owned(),
            address: "192.168.1.42".to_owned(),
            model: Some("yeelink.light.color3".to_owned()),
        };

        store.save(std::slice::from_ref(&bulb))?;
        let loaded = store.load()?;
        assert_eq!(loaded, vec![bulb]);

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = std::fs::metadata(&path)?;
            assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
        }
        Ok(())
    }

    #[test]
    fn a_file_with_a_token_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("bulbs.json");
        let store = BulbStore::new(path.clone());
        std::fs::write(
            &path,
            r#"{"version":1,"bulbs":[{"id":"00000000-0000-4000-8000-000000000001","name":"Desk lamp","address":"192.168.1.42","token":"0123456789abcdef0123456789abcdef"}]}"#,
        )?;
        assert!(matches!(store.load(), Err(StoreError::Decode { .. })));

        std::fs::write(
            &path,
            r#"{"version":1,"bulbs":[],"token":"0123456789abcdef0123456789abcdef"}"#,
        )?;
        assert!(matches!(store.load(), Err(StoreError::Decode { .. })));
        Ok(())
    }

    #[test]
    fn a_file_with_duplicate_names_or_addresses_is_rejected()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("bulbs.json");
        let store = BulbStore::new(path.clone());
        std::fs::write(
            &path,
            r#"{"version":1,"bulbs":[{"id":"00000000-0000-4000-8000-000000000001","name":"Desk lamp","address":"192.168.1.42"},{"id":"00000000-0000-4000-8000-000000000002","name":"Desk LAMP","address":"192.168.1.43"}]}"#,
        )?;
        assert!(matches!(
            store.load(),
            Err(StoreError::Invalid { message, .. }) if message.contains("two bulbs named")
        ));

        std::fs::write(
            &path,
            r#"{"version":1,"bulbs":[{"id":"00000000-0000-4000-8000-000000000001","name":"Desk lamp","address":"192.168.1.42"},{"id":"00000000-0000-4000-8000-000000000002","name":"Shelf lamp","address":"192.168.1.42"}]}"#,
        )?;
        assert!(matches!(
            store.load(),
            Err(StoreError::Invalid { message, .. }) if message.contains("two bulbs at")
        ));
        Ok(())
    }

    #[test]
    fn a_file_with_a_duplicate_id_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("bulbs.json");
        let store = BulbStore::new(path.clone());
        std::fs::write(
            &path,
            r#"{"version":1,"bulbs":[{"id":"00000000-0000-4000-8000-000000000001","name":"Desk lamp","address":"192.168.1.42"},{"id":"00000000-0000-4000-8000-000000000001","name":"Shelf lamp","address":"192.168.1.43"}]}"#,
        )?;
        assert!(matches!(store.load(), Err(StoreError::DuplicateId { .. })));
        Ok(())
    }

    #[test]
    fn preferences_round_trip_and_a_broken_file_falls_back()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("prefs.json");
        let store = PreferencesStore::new(path.clone());
        store.save(Preferences {
            sync_gain: 1.5,
            colour_intensity: 60.0,
            appearance: Appearance::Dark,
        })?;
        assert_eq!(store.load().sync_gain, 1.5);
        assert_eq!(store.load().colour_intensity, 60.0);
        assert_eq!(store.load().appearance, Appearance::Dark);

        std::fs::write(&path, "{ broken")?;
        assert_eq!(store.load(), Preferences::default());
        Ok(())
    }

    #[test]
    fn older_preferences_without_intensity_still_load() -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("prefs.json");
        let store = PreferencesStore::new(path.clone());
        std::fs::write(&path, r#"{"version":1,"syncGain":1.3}"#)?;
        let preferences = store.load();
        assert_eq!(preferences.sync_gain, 1.3);
        assert_eq!(preferences.colour_intensity, 0.0);
        assert_eq!(preferences.appearance, Appearance::System);
        Ok(())
    }

    #[test]
    fn appearance_maps_between_indices_and_the_system_flag() {
        assert!(Appearance::Dark.dark(false));
        assert!(!Appearance::Light.dark(true));
        assert!(Appearance::System.dark(true));
        assert!(!Appearance::System.dark(false));
        for appearance in [Appearance::Light, Appearance::Dark, Appearance::System] {
            assert_eq!(Appearance::from_index(appearance.index()), appearance);
        }
        assert_eq!(Appearance::from_index(9), Appearance::System);
    }

    #[test]
    fn unknown_appearance_falls_back_to_the_defaults() -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("prefs.json");
        let store = PreferencesStore::new(path.clone());
        std::fs::write(
            &path,
            r#"{"version":1,"syncGain":1.3,"appearance":"sepia"}"#,
        )?;
        assert_eq!(store.load(), Preferences::default());
        Ok(())
    }
}
