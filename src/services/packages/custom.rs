use crate::{
    blender::{Blender, BlenderError, ComputerGraphicsProgram},
    services::packages::{package::PackageT, BlenderPath},
};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Design to let user upload path to blender executables.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Custom {
    version: Version,
    executable: PathBuf,
}

impl Custom {

    fn new(version: Version, executable: impl AsRef<Path>) -> Self {
        Custom {
            version,
            executable: executable.as_ref().to_path_buf()
        }
    }

    #[allow(dead_code)]
    pub fn try_from(path: impl AsRef<Path>) -> Result<Self, BlenderError> {
        let blender = Blender::from_executable(path)?;
        let version = blender.get_version().to_owned();
        let executable = blender.get_executable().to_owned(); 
        Ok(Custom::new(version, executable))
    }
}

impl BlenderPath for Custom {
    fn get_blender(&self) -> Option<Blender> {
        Blender::from_executable(&self.executable).ok()
    }
}

impl PackageT for Custom {
    fn get_version(&self) -> &Version {
        &self.version
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

use super::*;

    // #[test]
    // fn assure_new_succeed() {
    //     // TODO: Find a way to fake this without having to download blender from the web.
    // }

    fn mock_custom(version: Option<Version>, executable: Option<PathBuf>) -> Custom {
        let version = version.unwrap_or(Version::new(4,2,0));
        let default_path = fs::canonicalize(PathBuf::from("./")).expect("Should be valid!");
        let executable = executable.unwrap_or(default_path);
        
        Custom::new(version, executable)
    }

    #[test]
    fn assure_get_version_succeed() {
        let version = Version::new(4, 2, 0);
        let mock = mock_custom(Some(version.clone()), None);
        let get_version = mock.get_version();
        assert_eq!(get_version, &version);
    }

    #[test]
    fn assure_invalid_blender_path_errors() {
        // try_from() should throw error on invalid file path
        let path = fs::canonicalize(PathBuf::from("./")).expect("Should be valid");
        let version = Version::new(4,2,0);
        let custom = Custom::try_from(path.clone());
        assert!(custom.is_err());
        
        // get_blender() should return none for invalid file path 
        let custom = Custom::new(version, path);
        let blender = custom.get_blender();
        assert!(blender.is_none());
    }
}
