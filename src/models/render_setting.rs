use super::{blender_scene::Sample, border::Border, format::Format};
use crate::blender::Frame;
use serde::{Deserialize, Serialize};
use std::{num::NonZero, path::PathBuf};

pub type FrameRate = NonZero<u16>; // u32 convert into string for xml-rpc. BEWARE!

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderSetting {
    /// output of where our stored image will save to
    pub output: PathBuf,
    /// Render frame Width
    pub width: Frame, // Not to be confused with animation frame
    /// Render frame height
    pub height: Frame, // Not to be confused with animation frame
    /// Samples capture from the scene
    pub sample: Sample,
    /// Frame per second
    #[serde(rename = "FPS")]
    pub fps: FrameRate,
    /// Image format
    pub format: Format,
    /// Borders
    pub border: Border,
}

impl RenderSetting {
    pub fn new(
        output: PathBuf,
        width: Frame,
        height: Frame,
        sample: Sample,
        fps: FrameRate,
        format: Format,
        border: Border,
    ) -> Self {
        Self {
            output,
            width,
            height,
            sample,
            fps,
            format,
            border,
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    pub fn mock_rendering_setting() -> RenderSetting {
        let sample = NonZero::new(100).unwrap();
        let fps = NonZero::new(30).unwrap();
        RenderSetting {
            output: PathBuf::new(),
            width: 1280,
            height: 720,
            sample,
            fps,
            format: Format::default(),
            border: Border::default(),
        }
    }
}
