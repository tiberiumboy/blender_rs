use super::{
    args::HardwareMode,
    blender_scene::{BlenderScene, Sample},
    device::Processor,
    format::Format,
};
use crate::blender::Frame;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::thread::available_parallelism;
use std::{io::Result as IoResult, num::NonZero};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
// This is a struct for python configuration when launch Blender
// We serialize this into JSON and pass it into command arguments
// On python side; The JSON gets decode and use the information stored
//      to apply settings directly to Blender before rendering.
pub struct BlenderConfiguration {
    /// Exact output path
    output: PathBuf,
    scene_info: BlenderScene,
    /// The number of cores to utilize for this rendering job.
    cores: NonZero<usize>, // ensure that this value will never be zero.
    /// Which rendering architecture to use
    processor: Processor,
    /// Which hardware to utilize
    hardware_mode: HardwareMode,
    /// The sample count (Overrides the default blend file settings)
    sample: Sample,
    /// Rendered image format
    format: Format,
    /// Render starts from this frame (Inclusive)
    start: Frame,
    /// Render completes after this frame (Inclusive)
    end: Frame,
    // Py:- Value assign to use_crop_to_border, additionally, false set film_transparent true
    crop: bool,
}

impl BlenderConfiguration {
    fn new(
        output: PathBuf,
        scene_info: BlenderScene,
        cores: NonZero<usize>,
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
        let cores = available_parallelism()?;
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
            NonZero::new(1).unwrap(),
            Format::default(),
            0,
            1,
        );
        assert!(config.is_ok());
    }
}
