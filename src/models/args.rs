/*
    Developer blog

    - Having done extensive research, Blender have two ways to interface to the program
        1. Through CLI
        2. Through Python API via "bpy" library

    Review online for possible solution to interface blender via CAPI, but was strongly suggested to use a python script instead
    this limits what I can do in term of functionality, but it'll be a good start.
    FEATURE - See if python allows pointers/buffer access to obtain job render progress - Allows node to send host progress result. Possibly viewport network rendering?

    Do note that blender is open source - it's not impossible to create FFI that interfaces blender directly, but rather, there's no support to perform this kind of action (yet).
*/
// May Subject to change.
use super::device::Processor;
use crate::{
    blend_file::BlendFile,
    blender::{BlenderError, Frame},
    models::{config::BlenderConfiguration, format::Format, peek_response::PeekResponse},
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Result as IoResult,
    path::{Path, PathBuf},
};

// Blender 4.2 introduce a new enum called BLENDER_EEVEE_NEXT, which is currently handle in python file atm.
// const EEVEE_SWITCH: Version = Version::new(4, 2, 0);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HardwareMode {
    CPU,
    GPU,
    BOTH,
}

// ref: https://docs.blender.org/manual/en/latest/advanced/command_line/render.html
/// Field must be public to offer context to render the scene.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Args {
    pub file: BlendFile, // required
    pub output: PathBuf,
    pub processor: Processor,
    pub mode: HardwareMode, // optional
    pub format: Format,     // optional - default to Png
    pub start: Frame,
    pub end: Frame,
}

impl Args {
    pub fn new(file: BlendFile, output: PathBuf, start: Frame, end: Frame) -> Self {
        Args {
            file,
            output,
            processor: Processor::NONE,
            mode: HardwareMode::CPU,
            format: Format::default(),
            start,
            end,
        }
    }

    /// Args are user provided value
    /// Generates python configuration structure
    pub(crate) fn generate_blender_config(&self) -> IoResult<BlenderConfiguration> {
        // part of this file doesn't make a lot of sense here?
        // what happen here?
        let info: PeekResponse = self.file.peek_response();
        BlenderConfiguration::create(
            self.output.clone(),
            info.current.clone(),
            self.processor.clone(),
            self.mode.clone(),
            info.current.render_setting.sample,
            info.current.render_setting.format,
            self.start,
            self.end,
        )
    }

    pub(crate) fn generate_arg_command(
        &self,
        script_path: impl AsRef<Path>,
    ) -> Result<Vec<String>, BlenderError> {
        let settings = self
            .generate_blender_config()
            .map_err(BlenderError::IoError)?;

        let path = self.file.to_path().as_os_str().to_os_string();
        // provide the configuration in json format
        let content = serde_json::to_string(&settings)
            .map_err(|e| BlenderError::InvalidFile(e.to_string()))?;

        let script_path = script_path
            .as_ref()
            .to_str()
            .ok_or(BlenderError::ExecutableInvalid)?;
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
            script_path.into(),
            "--".into(),
            "-c".into(),
            // TODO: does this handle escaped characters?
            content,
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blend_file::tests::mock_blend_file;

    fn mock_args() -> Args {
        let default_example_path = PathBuf::from("./examples/assets/test.blend");
        let file = BlendFile::try_from(default_example_path).expect("Unable to find example file!");
        let output = fs::canonicalize(PathBuf::from("./examples/assets/"))
            .expect("Must be able to collapse to absolute path!");
        Args::new(file, output, 1, 2)
    }

    #[test]
    fn success_args_builder() {
        let mock_blend_file = mock_blend_file();
        // Must have blendfile exist and verify integrity usage.
        let start = 1;
        let end = 2;
        let output = PathBuf::new(); // TOOD: Find a way to reference ./blender_rs/examples/assets/test.blend without reference/abs path chaoticness
        let args = Args::new(mock_blend_file.clone(), output.clone(), start, end);

        assert_eq!(args.file, mock_blend_file);
        assert_eq!(args.output, output);
        assert_eq!(args.start, start);
        assert_eq!(args.end, end);
    }

    #[test]
    fn assure_generate_blender_config_succeed() {
        let args = mock_args();
        let blender_config = args.generate_blender_config();
        assert!(blender_config.is_ok());
    }

    #[test]
    fn assure_generate_arg_command_succeed() {
        let args = mock_args();
        let col = args.generate_arg_command(PathBuf::new());
        assert!(col.is_ok());
    }
}
