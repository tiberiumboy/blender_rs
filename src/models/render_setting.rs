use super::{blender_scene::Sample, format::Format, border::Border};
use crate::blender::Frame;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub type FrameRate = u16; // u32 convert into string for xml-rpc. BEWARE!

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
        RenderSetting {
            output: PathBuf::new(),
            width: 1280,
            height: 720,
            sample: 100,
            fps: 30,
            format: Format::default(),
            border: Border::default(),
        }
    }
}
