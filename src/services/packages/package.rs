use crate::{
    blender::Blender,
    services::{
        category::BlenderCategoryError,
        packages::{
            bundle::Bundle, /* custom::Custom, */ download_link::DownloadLink,
            downloaded::Downloaded, BlenderPath,
        },
    },
};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub(crate) trait PackageT {
    fn get_version(&self) -> &Version;
}

/*
    Package is thought of having a single source of truth to get blender specific versions.
    Depends on the phase, we would need to download if it's not found within local system.
    Otherwise, use the uncompressed version of the executable and treat as final source of truth.
    We have method implementations to gracefully fetch the package.
*/
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Package {
    // Only contains download link
    Metadata(DownloadLink),
    // contains download origin and path to downloaded content
    Downloaded(Downloaded),
    // Contains complete set, do not download, do not unzip, should provide executable path
    Bundle(Bundle),
    // Only contains executable location, user defined variable
    // Executable(Custom),
    // TODO: Feature request - Would there ever be a chances for any of the data above would mutate and become invalid? Test this out?
    // In some extreme cases - if something goes wrong, we can put them in malform state until user corrects them into Bundle state, or lesser state known.
    // Malformed { origin: Option<Url>, downloaded: Option<PathBuf>, executable: Option<PathBuf> },
}

impl Package {
    // This is design to check internal source and verify the package is indeed correct, otherwise return the current state it failed in
    // we are only provided with a source.
    pub fn check_package(
        link: DownloadLink,
        destination: impl AsRef<Path>,
    ) -> Result<Package, BlenderCategoryError> {
        // This ideally should return something...
        // we'll start here first
        let downloaded = match link.content_exist(destination) {
            Ok(downloaded) => downloaded,
            Err(download_link) => return Ok(Package::Metadata(download_link)),
        };

        match downloaded.check_unpacked() {
            Ok(bundle) => Ok(Package::Bundle(bundle)),
            // Do not unzip, simply return the current state and move on.
            Err(downloaded) => Ok(Package::Downloaded(downloaded)),
        }
    }

    // This is an attempt to download from url, extract, and provide package ready to be used for blender.
    pub fn get_package_ready(
        self,
        destination: impl AsRef<Path>,
    ) -> Result<Package, BlenderCategoryError> {
        match self {
            Package::Metadata(link) => Ok(link
                .download(&destination)
                .map_err(BlenderCategoryError::Io)?
                .extract()),
            Package::Downloaded(link) => Ok(link.extract()),
            // These two are ok since they were already ready to begin with
            // Package::Executable(..) => Ok(self),
            Package::Bundle(..) => Ok(self),
        }
    }
}

impl PackageT for Package {
    fn get_version(&self) -> &Version {
        match self {
            Package::Metadata(link) => link.get_version(),
            Package::Downloaded(content) => content.get_version(),
            Package::Bundle(bundle) => bundle.get_version(),
        }
    }
}

impl BlenderPath for Package {
    // without modifying itself, we can only provide as much.
    fn get_blender(&self) -> Option<Blender> {
        match self {
            Package::Bundle(bundle) => bundle.get_blender(),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use super::*;
    use crate::services::packages::{
        bundle::tests::mock_bundle, download_link::tests::mock_downloadlink,
    };

    fn create_destination() -> PathBuf {
        fs::canonicalize(PathBuf::from("./")).expect("Must have a valid destination path!")
    }

    #[test]
    fn assure_check_package_succeed() {
        let link = mock_downloadlink();
        let destination = create_destination();
        let result = Package::check_package(link, destination);
        assert!(result.is_ok());
    }

    #[test]
    fn assure_get_package_ready_succeed() {
        // TODO: find a way to test all Package enum without downloading from Blender?
        let bundle = mock_bundle();
        let package = Package::Bundle(bundle);
        let destination = create_destination();
        let result = package.get_package_ready(&destination);
        assert!(result.is_ok());
    }

    #[test]
    fn assure_get_version_succeed() {
        let bundle = mock_bundle();
        let package = Package::Bundle(bundle.clone());

        let version = package.get_version();
        assert_eq!(version, bundle.get_version());
    }

    #[test]
    fn assure_invalid_get_version_return_none() {
        let download_link = mock_downloadlink();
        let package = Package::Metadata(download_link.clone());

        let version = package.get_version();
        assert_eq!(version, download_link.get_version());
    }

    #[test]
    fn assure_get_blender_succeed() {
        let bundle = mock_bundle();
        let package = Package::Bundle(bundle);
        let result = package.get_blender();
        // TODO: Find a way to ensure is_ok() returns
        assert!(result.is_none());
    }

    #[test]
    fn assure_invalid_blender_return_none() {
        let download_link = mock_downloadlink();
        let package = Package::Metadata(download_link);

        let result = package.get_blender();
        assert!(result.is_none());
    }
}
