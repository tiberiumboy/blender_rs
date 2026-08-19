use crate::blender::{Blender, ComputerGraphicsProgram};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

// could I use this to describe in a TOML/YAML/JSON file?
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlenderConfig {
    /// List of installed blenders
    blenders: HashMap<Version, Blender>,

    /// Installation path. By default set to `$HOME/Downloads/Blender`
    install_path: PathBuf,
}

impl BlenderConfig {
    pub fn get_download_destination(&self, category_folder_name: &str) -> PathBuf {
        self.install_path.join(category_folder_name)
    }

    // Fetch best matching version of blender if provided, or latest version available if none was provided.
    pub fn get_latest_blender_available(&self, version: &Version) -> Option<&Blender> {
        self.get_blender(version)
            .or_else(|| self.get_blender_partial(version.major, version.minor))
    }

    /// Return matching exact blender version
    pub fn get_blender(&self, version: &Version) -> Option<&Blender> {
        self.blenders.values().find(|x| x.get_version().eq(version))
    }

    // return a immutable reference list of installed blender.
    // useful to display on website of some sort.
    pub fn get_blenders(&self) -> Vec<&Blender> {
        self.blenders
            .iter()
            .fold(Vec::new(), |mut map, (_, blender)| {
                map.push(blender);
                map
            })
    }

    pub fn set_install_path(&mut self, new_path: impl AsRef<Path>) {
        self.install_path = new_path.as_ref().to_path_buf();
    }

    /// Return a reference to matching partial version, but uses latest patch
    /// Major must match, Minor will match if greater than 0. Patch will always be the latest version possible.
    pub(crate) fn get_blender_partial(&self, major: u64, minor: u64) -> Option<&Blender> {
        self.blenders
            .values()
            .fold(None, |latest: Option<&Blender>, item| {
                let current_version = item.get_version();

                if current_version.major.ne(&major) {
                    return latest;
                }

                // custom rule: If minor = 0 (default), use latest, otherwise compare all others.
                if minor > 0 && current_version.minor.ne(&minor) {
                    return latest;
                }

                if let Some(recent) = latest {
                    if recent.get_version().ge(current_version) {
                        return latest;
                    }
                }

                Some(item)
            })
    }

    /// Remove any invalid blender path entry from BlenderConfig
    pub fn remove_invalid_blender(&mut self) {
        self.blenders.retain(|_, v| v.get_executable().exists());
    }

    /// remove target blender
    pub fn remove_blender(&mut self, blender: &Blender) -> Option<Blender> {
        self.blenders.remove(blender.get_version())
    }

    /// Append blender entry to database
    /// This will create a new record if the key does not exist, or update record, returning old value.
    pub fn insert_blender(&mut self, blender: &Blender) -> Option<Blender> {
        // If Some returns, it means we override record. None means no previous record exist and a new entry is added.
        self.blenders
            .insert(blender.get_version().to_owned(), blender.clone())
    }
}

impl Default for BlenderConfig {
    fn default() -> Self {
        // TODO: Change this so it's not always depends on download_dir() by default. For now, default to download location.
        let install_path = dirs::download_dir()
            .expect("Must have place to download!")
            .join("BlendFarm/Blenders");

        // ensure path location must exist to save and store to
        // - we've been given a place with permission access.
        if let Err(e) = fs::create_dir_all(&install_path) {
            eprintln!("Unable to create {e:?}");
        }

        Self {
            blenders: Default::default(),
            install_path,
        }
    }
}

impl Into<PathBuf> for BlenderConfig {
    fn into(self) -> PathBuf {
        self.install_path
    }
}

#[cfg(test)]
pub mod tests {
    use crate::blender::test::mock_blender;

    use super::*;

    pub fn mock_blender_config(blender: Option<Version>) -> BlenderConfig {
        let mut blenders = HashMap::new();

        if let Some(version) = blender {
            let blender = mock_blender(None, version.clone());
            blenders.insert(version, blender);
        }

        // TODO: Find a way to mock these properties?
        let install_path = PathBuf::new();
        BlenderConfig {
            blenders,
            install_path,
        }
    }

    #[test]
    fn assure_get_download_destination_succeed() {
        let config = mock_blender_config(None);
        let get_destination = config.get_download_destination("category_folder_name".into());
        assert_eq!(
            config.install_path.join("category_folder_name"),
            get_destination
        );
    }

    #[test]
    fn assure_get_latest_blender_available_succeed() {
        let version = Version::new(4, 0, 0);
        let config = mock_blender_config(Some(version.clone()));

        let result = config.get_latest_blender_available(&version);
        assert!(result.is_some_and(|b| b.get_version().eq(&version)));

        let result = config.get_latest_blender_available(&Version::new(4, 1, 0));
        assert!(result.is_none());

        let result = config.get_latest_blender_available(&Version::new(4, 0, 1));
        assert!(result.is_some());
    }

    #[test]
    fn assure_get_blender_succeed() {
        let version = Version::new(4, 0, 0);
        let config = mock_blender_config(Some(version.clone()));

        let result = config.get_blender(&version);
        assert!(result.is_some());

        let result = config.get_blender(&Version::new(4, 0, 1));
        assert!(result.is_none());
    }

    #[test]
    fn assure_insert_blender_succeed() {
        let version = Version::new(4, 0, 0);
        let blender = mock_blender(None, version.clone());
        let mut config = mock_blender_config(None);

        config.insert_blender(&blender);
        assert!(config.get_blender(&version).is_some());
    }

    #[test]
    fn assure_remove_blender() {
        let version = Version::new(4, 0, 0);
        let blender = mock_blender(None, version.clone());
        let mut config = mock_blender_config(Some(version.clone()));

        config.remove_blender(&blender);

        assert!(config.blenders.iter().count() == 0);
    }

    #[test]
    fn assure_remove_invalid_blender() {
        // append a invalid blender version
        let mut config = mock_blender_config(Some(Version::new(4, 0, 0)));
        // call to remove invalid blender
        config.remove_invalid_blender();
        // should hold no blender in mockup
        assert!(config.blenders.iter().count() == 0);
    }

    #[test]
    fn assure_get_blender_partial_success() {
        let major = 4;
        let minor = 1;
        let version = Version::new(major, minor, 1);
        // TODO: Add more blender version and run unit test for version greater than comparison in major and minor
        let config = mock_blender_config(Some(version.clone()));

        let result = config.get_blender_partial(major, minor);
        assert!(result.is_some());

        let result = config.get_blender_partial(major, minor + 1);
        assert!(result.is_none());

        let result = config.get_blender_partial(major, minor - 1);
        assert!(result.is_some());
    }

    #[test]
    fn assure_get_blenders_success() {
        let version = Version::new(4, 0, 1);
        let config = mock_blender_config(Some(version.clone()));
        let mut list = config.get_blenders();
        assert!(list.pop().is_some_and(|b| b.get_version().eq(&version)));
    }

    #[test]
    fn assure_set_install_path_succeed() {
        let mut config = mock_blender_config(None);
        let new_install_location = PathBuf::from("/tmp");
        config.set_install_path(new_install_location.clone());
        assert!(config.install_path.eq(&new_install_location));
    }

    #[test]
    fn assure_blender_config_default_succeed() {
        let config = BlenderConfig::default();
        assert_eq!(config.blenders, HashMap::new());
        assert!(config.install_path.exists());
    }

    #[test]
    fn assure_into_path_succeed() {
        let config = mock_blender_config(None);
        let path: PathBuf = config.clone().into();
        assert_eq!(path, config.install_path);
    }
}
