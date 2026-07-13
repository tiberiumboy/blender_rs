use crate::{
    blender::BlenderError,
    models::{
        border::Border, format::Format, peek_response::PeekResponse, render_setting::RenderSetting,
        scene_info::SceneInfo,
    },
};
use blend::Blend;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// A struct to hold valid blend file with compatible partial version.
// we can extract additional data if we need to?
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlendFile {
    inner: PathBuf,
    major: u16,
    minor: u16,
    scene_info: SceneInfo,
    render_setting: RenderSetting,
}

impl BlendFile {
    pub fn try_from(blend_file_path: impl AsRef<Path>) -> Result<Self, BlenderError> {
        let file_path = blend_file_path.as_ref();

        // .expect() was found and called if the file does not exist inside blend crate.
        // Instead of creating a pull request, the crate expects application developer to verify the file integrity and validation beforehand.
        // Throw an error here if the file do not exist.
        if !file_path.exists() || !file_path.is_file() {
            return Err(BlenderError::InvalidFile(
                "Blend file not found!".to_owned(),
            ));
        }

        // An issue here where Blend::from_path will panic if the provided file path do not exist or invalid. It is gracefully handle in the line above.
        let blend = Blend::from_path(file_path).map_err(|e| {
            BlenderError::InvalidFile(format!("Received BlendParseError! {e:?}").to_owned())
        })?;

        let version = &blend.blend.header.version;
        let major = version.major;
        let minor = version.minor;
        // TODO: Where/how do we load format and windows from?
        let format = Format::default();
        let window = Border::default();

        let scene_info = SceneInfo::process(&blend)?;
        let render_setting = scene_info.clone().render_setting(format, window);
        let inner = blend_file_path.as_ref().to_path_buf();

        Ok(BlendFile::new(
            inner,
            major,
            minor,
            scene_info,
            render_setting,
        ))
    }

    fn new(
        inner: PathBuf,
        major: u16,
        minor: u16,
        scene_info: SceneInfo,
        render_setting: RenderSetting,
    ) -> Self {
        BlendFile {
            inner,
            major,
            minor,
            render_setting,
            scene_info,
        }
    }

    pub fn get_partial_version(&self) -> (u16, u16) {
        (self.major, self.minor)
    }

    pub fn peek_response(&self) -> PeekResponse {
        let last_version = Version::new(self.major.into(), self.minor.into(), 0);
        self.scene_info.peek_response(last_version)
    }

    pub fn to_path(&self) -> &Path {
        self.inner.as_path()
    }
}

impl Into<PathBuf> for BlendFile {
    fn into(self) -> PathBuf {
        self.inner
    }
}

impl Into<RenderSetting> for BlendFile {
    fn into(self) -> RenderSetting {
        self.render_setting
    }
}

impl Into<SceneInfo> for BlendFile {
    fn into(self) -> SceneInfo {
        self.scene_info
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::models::render_setting::tests::mock_rendering_setting;
    use crate::models::scene_info::tests::mock_scene_info;
    use std::fs::File;
    use std::str::FromStr;
    use std::{env, fs};

    pub(crate) fn mock_blend_file() -> BlendFile {
        let mut dir = env::current_exe().expect("Must have valid current executable!");
        dir.pop();
        dir.pop();
        dir.pop();
        dir.pop();
        // TODO: Find a way to reference relative path to ./blender_rs/examples/assets/test.blend?
        dbg!(&dir);
        let example_blend = "./examples/assets/test.blend";
        let scene_info = mock_scene_info();
        let render_setting = mock_rendering_setting();
        let inner = PathBuf::from_str(example_blend).expect("Must have a valid location!");
        BlendFile {
            inner,
            major: 4,
            minor: 2,
            scene_info,
            render_setting,
        }
    }

    pub(crate) fn get_default_example_path() -> PathBuf {
        let file = PathBuf::from("./examples/assets/test.blend");
        if !file.exists() {
            panic!(
                "Example file do not exist! Please do not remove the example file from the repo!"
            )
        }
        file
    }

    #[test]
    fn assure_blend_file_succeed_with_example() {
        let good_file = BlendFile::try_from(get_default_example_path());
        assert!(good_file.is_ok());
    }

    #[test]
    fn assure_empty_blend_file_returns_invalid_file() {
        let path = fs::canonicalize(PathBuf::from("./"))
            .expect("Must be able to resolve absolute file path!")
            .join("test.blend");
        File::create(path.clone()).expect("Must be able to create fake file for this test");

        let bad_file = BlendFile::try_from(path.clone());
        assert!(bad_file.is_err());

        fs::remove_file(path).expect("Should be able to clean up test files");
    }

    #[test]
    fn assure_get_partial_version_succeed() {
        let mock = mock_blend_file();
        let (major, minor) = mock.get_partial_version();
        assert_eq!(major, mock.major);
        assert_eq!(minor, mock.minor);
    }

    #[test]
    fn assure_blend_file_existance_fails() {
        let bad_file = BlendFile::try_from(PathBuf::new()); // should fail.
        assert!(bad_file.is_err());
    }

    #[test]
    fn assure_blend_file_only() {
        let mut example_path = get_default_example_path();
        // replace .blend to .txt
        // then call the function to make sure that blend service rejects non .blend formats.
        assert!(example_path.set_extension(".txt".to_owned()));

        let blend_file = BlendFile::try_from(example_path);
        assert!(blend_file.is_err());
    }

    #[test]
    fn assure_into_pathbuf_succeed() {
        let mock = mock_blend_file();
        let path_buf: PathBuf = mock.clone().into();
        assert_eq!(path_buf, mock.inner);
    }

    #[test]
    fn assure_into_render_settings_succeed() {
        let mock = mock_blend_file();
        let render_setting: RenderSetting = mock.into();
        assert_eq!(mock_rendering_setting(), render_setting);
    }

    #[test]
    fn assure_into_scene_info_succeed() {
        let mock = mock_blend_file();
        let scene_info: SceneInfo = mock.into();
        assert_eq!(mock_scene_info(), scene_info);
    }
}
