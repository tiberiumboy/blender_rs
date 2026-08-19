use crate::{
    blender::{Blender, ComputerGraphicsProgram},
    services::packages::{downloaded::Downloaded, package::PackageT, BlenderPath},
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Bundle {
    pub content: Downloaded,
    executable: PathBuf,
}

impl Bundle {
    pub(crate) fn new(content: Downloaded, executable: PathBuf) -> Self {
        Self {
            content,
            executable,
        }
    }
}

impl BlenderPath for Bundle {
    fn get_blender(&self) -> Option<Blender> {
        Blender::from_executable(&self.executable).ok()
    }
}

impl PackageT for Bundle {
    fn get_version(&self) -> &semver::Version {
        &self.content.origin.version
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::services::packages::downloaded::tests::mock_downloaded;

    pub(crate) fn mock_bundle() -> Bundle {
        let download = mock_downloaded();
        let path = download.content.clone();
        Bundle::new(download, path)
    }

    #[test]
    fn assure_new_succeed() {
        let download = mock_downloaded();
        let executable = PathBuf::new();
        let bundle = Bundle::new(download.clone(), executable.clone());

        assert!(bundle.content.eq(&download));
        assert!(bundle.executable.eq(&executable));
    }

    #[test]
    fn assure_get_blender_succeed() {
        let bundle = mock_bundle();
        let result = bundle.get_blender();
        assert!(result.is_none());
    }

    #[test]
    fn assure_get_version_succeed() {
        let bundle = mock_bundle();
        let result = bundle.get_version();
        assert!(result.eq(bundle.get_version()));
    }
}
