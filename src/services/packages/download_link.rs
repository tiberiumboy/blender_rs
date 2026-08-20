use crate::services::{
    category::BlenderCategoryError,
    packages::{downloaded::Downloaded, package::PackageT},
};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Error as IoError,
    path::{Path, PathBuf},
};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct DownloadLink {
    version: Version,
    pub file_name: String, // contains extensions!
    pub download_url: Url,
}

impl DownloadLink {

    fn new(file_name: String, download_url: Url, version: Version) -> DownloadLink {
        Self {
            file_name,
            download_url,
            version
        }
    }

    pub fn from(url: Url, version: Version) -> Result<DownloadLink, BlenderCategoryError> {
        let name = url
            .path_segments()
            .ok_or(BlenderCategoryError::NotFound)?
            .last()
            .ok_or(BlenderCategoryError::NotFound)?
            .to_owned();

        Ok(Self::new(name,url,version))
    }

    #[inline]
    fn download_path(&self, install_path: impl AsRef<Path>) -> PathBuf {
        install_path.as_ref().join(&self.file_name)
    }

    // Destination expects absolute path
    pub fn content_exist(self, destination: impl AsRef<Path>) -> Result<Downloaded, DownloadLink> {
        let path = self.download_path(destination);
        if path.exists() {
            let downloaded = Downloaded::new(self, path);
            return Ok(downloaded);
        }
        Err(self)
    }

    // at this point here we will download the link and return an updated state
    pub fn download(self, destination: impl AsRef<Path>) -> Result<Downloaded, IoError> {
        // got a permission denied here? Interesting?
        // I need to figure out why and how I can stop this from happening?
        fs::create_dir_all(&destination)?;

        // create a target name
        let target = self.download_path(destination);

        // Check and see if we haven't download the file already
        if !target.exists() {
            // Here we make the call to attohttpc
            // Download the file from the internet
            let response = attohttpc::get(self.download_url.as_str())
                .send()
                .map_err(IoError::other)?;
            // TODO: See if there's a better way to save or store the file?
            // It's like why can't we stream directly to io?
            match response.bytes() {
                Ok(data) => fs::write(&target, data)?,
                Err(e) => eprintln!("Fail to read data from response! {e:?}"),
            }
        }

        // Assume the file we download are zipped/compressed.
        Ok(Downloaded {
            origin: self,
            content: target,
        })
    }
}

impl PackageT for DownloadLink {
    fn get_version(&self) -> &Version {
        &self.version
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn mock_downloadlink() -> DownloadLink {
        let version = Version::new(4, 0, 1);
        let file_name = "test_file.txt".to_owned();
        let example = fs::canonicalize(PathBuf::from("./")).unwrap();
        let download_url = Url::from_file_path(example.clone()).unwrap();
        DownloadLink::new(file_name, download_url, version)
    }

    #[test]
    fn assure_new_succeed() {
        let mock = mock_downloadlink();
        assert!(mock.version.eq(&Version::new(4, 0, 1)));
    }

    #[test]
    fn assure_download_path_succeed() {
        let mock = mock_downloadlink();
        let path = mock.download_path(mock.download_url.as_str());
        assert_eq!(
            PathBuf::from(mock.download_url.to_string()).join(mock.file_name),
            path
        );
    }

    // TODO: before uncommenting below - find a way to mock attohttpc for unit test purposes
    // #[test]
    // fn assure_download_succeed() {
    //     let mock = mock_downloadlink();
    //     let destination = dirs::download_dir().expect("Must have a path to download!");
    //     let destination = destination.join("Blender/");
    //     let result = mock.download(destination);
    //     assert!(result.is_ok());
    // }

    #[cfg(target_os = "macos")]
    #[test]
    fn assure_copy_dir_all_succeed() {}
}
