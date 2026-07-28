use std::{
    fs,
    hash::{DefaultHasher, Hasher},
    num::ParseIntError,
    path::{Path, PathBuf},
};

use blend::Blend;
use semver::Version;
use serde::{Deserialize, Serialize};

use crate::{
    blender::BlenderError,
    models::{
        config::BlenderConfiguration, peek_response::PeekResponse, render_setting::RenderSetting,
        scene_info::SceneInfo,
    },
    utils::get_config_folder_path,
};

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
    fn get_script_path() -> Result<PathBuf, BlenderError> {
        Ok(get_config_folder_path()
            .map_err(BlenderError::IoError)?
            .join("render.py"))
    }

    fn calculate_checksum(input: &[u8]) -> u64 {
        let mut hash = DefaultHasher::new();
        for bit in input {
            hash.write_u8(*bit);
        }
        hash.finish()
    }

    pub fn new(path_to_blend_file: impl AsRef<Path>) -> Result<Self, BlenderError> {
        let blend = Blend::from_path(&path_to_blend_file).map_err(|e| {
            BlenderError::InvalidFile(format!("Received BlenderParseError! {e:?}").to_owned())
        })?;

        // blender version are display as three digits number, e.g. 404 is major: 4, minor: 4.
        // treat this as a u16 major = u16 / 100, minor = u16 % 100;
        let str_version = std::str::from_utf8(&blend.blend.header.version)
            .map_err(|e| BlenderError::InvalidFile(e.to_string()))?;

        let value: u16 = str_version
            .parse()
            .map_err(|e: ParseIntError| BlenderError::InvalidFile(e.to_string()))?;
        let major = value / 100;
        let minor = value % 100;

        let scene_info = SceneInfo::default().process(&blend)?;
        let render_setting = scene_info.clone().render_setting();

        Ok(BlendFile {
            inner: path_to_blend_file.as_ref().to_path_buf(),
            major,
            minor,
            render_setting,
            scene_info,
        })
    }

    pub fn get_partial_version(&self) -> (u16, u16) {
        (self.major, self.minor)
    }

    pub fn peek_response(&self, version: Option<&Version>) -> PeekResponse {
        let last_version = match version {
            Some(v) => v,
            None => &Version::new(self.major.into(), self.minor.into(), 0),
        };
        self.scene_info.peek_response(last_version)
    }

    pub fn to_path(&self) -> &Path {
        self.inner.as_path()
    }

    pub fn setup_args(&self, settings: &BlenderConfiguration) -> Result<Vec<String>, BlenderError> {
        let script_path = Self::get_script_path()?;
        let data = include_bytes!("./render.py");
        if !script_path.exists() {
            fs::write(&script_path, data).map_err(BlenderError::IoError)?;
        } else {
            let content = fs::read(&script_path).map_err(BlenderError::IoError)?;
            let source = Self::calculate_checksum(data);
            let target = Self::calculate_checksum(&content);
            if source != target {
                fs::write(&script_path, data).map_err(BlenderError::IoError)?;
            }
        }

        let path = self.to_path().as_os_str().to_os_string();
        // provide the configuration in json format
        let content = serde_json::to_string(settings)
            .map_err(|e| BlenderError::InvalidFile(e.to_string()))?;

        Ok(vec![
            "--factory-startup".to_owned(),
            "-noaudio".into(),
            "-b".into(),
            fs::canonicalize(path)
                .unwrap()
                .to_str()
                .unwrap_or_default()
                .to_owned(),
            "-P".into(),
            script_path.to_str().unwrap().into(),
            "--".into(),
            "-c".into(),
            // does this handle escaped characters?
            content,
        ])
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
    use std::env;
    use std::str::FromStr;

    use super::*;
    use crate::models::render_setting::tests::mock_rendering_setting;
    use crate::models::scene_info::tests::mock_scene_info;

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
}
