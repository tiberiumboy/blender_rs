use crate::blender::Frame;

use super::{
    args::HardwareMode,
    blender_scene::{BlenderScene, Sample},
    device::Processor,
    format::Format,
};
use serde::{Deserialize, Serialize};
use std::io::Result as IoResult;
use std::path::PathBuf;
use std::thread::available_parallelism;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
// TODO: could rename this to something else? This is a struct to serialize into JSON for python configuration
pub struct BlenderConfiguration {
    // output various
    output: PathBuf,
    scene_info: BlenderScene,
    cores: usize,
    processor: Processor,
    hardware_mode: HardwareMode,
    sample: Sample,
    format: Format,
    start: Frame,
    end: Frame,
    // Py:- Value assign to use_crop_to_border, additionally, false set film_transparent true
    crop: bool,
}

impl BlenderConfiguration {
    fn new(
        output: PathBuf,
        scene_info: BlenderScene,
        cores: usize,
        processor: Processor,
        hardware_mode: HardwareMode,
        sample: Sample,
        format: Format,
        crop: bool,
        start: Frame,
        end: Frame,
    ) -> Self {
        BlenderConfiguration {
            output,
            scene_info,
            cores,
            processor,
            hardware_mode,
            sample,
            format,
            crop,
            start,
            end,
        }
    }

    // Create a configuration for Python script to run and utilize from.
    pub(crate) fn create(
        output: PathBuf,
        scene_info: BlenderScene,
        processor: Processor,
        hardware_mode: HardwareMode,
        sample: Sample,
        format: Format,
        start: Frame,
        end: Frame,
    ) -> IoResult<BlenderConfiguration> {
        // try to pull the core, or throw error
        let cores = available_parallelism()?.get();
        Ok(Self::new(
            output,
            scene_info,
            cores,
            processor,
            hardware_mode,
            sample,
            format,
            false,
            start,
            end,
        ))
    }
}

#[cfg(test)]
pub mod tests {

    use super::*;
    use crate::models::blender_scene::tests::mock_blender_scene;

    #[test]
    fn assure_create_succeed() {
        let blender_scene = mock_blender_scene();
        let config = BlenderConfiguration::create(
            PathBuf::new(),
            blender_scene,
            Processor::NONE,
            HardwareMode::BOTH,
            0,
            Format::default(),
            0,
            1,
        );
        assert!(config.is_ok());
    }
}
