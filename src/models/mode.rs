use crate::blender::BlenderError;
use serde::{Deserialize, Serialize};

// context for serde: https://serde.rs/enum-representations.html
#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize)]
pub enum RenderMode {
    // JSON: "Frame": "i32",
    // render a single frame.
    Frame(i32),

    // JSON: "Animation": {"start":"i32", "end":"i32"}
    // contains the target start frame to the end target frame.
    Animation { start: i32, end: i32 },
    // future project - allow network node to only render section of the frame instead of whole to visualize realtime rendering view solution.
    // JSON: "Section": {"frame":"i32", "coord":{"i32", "i32"}, "size": {"i32", "i32"} }
    // Section {
    //     frame: i32,
    //     coord: (i32, i32),
    //     size: (i32, i32),
    // },
}

impl RenderMode {
    pub fn new(start: i32, end: i32) -> RenderMode {
        let mut start = start;
        let mut end = end;

        // start needs to be the lowest number of all. If it's backward, flip it around.
        if start > end {
            (start, end) = (end, start);
        }

        if start == end {
            RenderMode::Frame(start)
        } else {
            RenderMode::Animation { start, end }
        }
    }

    pub fn try_new(start: &str, end: &str) -> Result<RenderMode, BlenderError> {
        // stop if the parser fail to parse.
        let start = start.parse::<i32>().map_err(BlenderError::ParseInt)?;
        let end = end.parse::<i32>().map_err(BlenderError::ParseInt)?;

        Ok(RenderMode::new(start, end))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assure_new_frame_succeed() {
        // matching start and end should provide frame struct
        let mode = RenderMode::new(1, 1);
        assert_eq!(mode, RenderMode::Frame(1));
    }

    #[test]
    fn assure_new_animation_mode_succeed() {
        // non-equal value should produce animation instead of frame
        let mode = RenderMode::new(1, 2);
        assert_eq!(mode, RenderMode::Animation { start: 1, end: 2 });

        // Should be able to correct the animation frame
        let mode = RenderMode::new(2, 1);
        assert_eq!(mode, RenderMode::Animation { start: 1, end: 2 });
    }

    #[test]
    // negative should work as well
    fn assure_negative_animation_number_succeed() {
        let mode = RenderMode::new(-1, -2);
        assert_eq!(mode, RenderMode::Animation { start: -2, end: -1 });
    }

    #[test]
    fn assure_try_new_succeed() {
        let mode = RenderMode::try_new("0", "0");
        assert!(mode.is_ok_and(|m| m.eq(&RenderMode::Frame(0))));

        let mode = RenderMode::try_new("0", "1");
        assert!(mode.is_ok_and(|m| m.eq(&RenderMode::Animation { start: 0, end: 1 })));

        let mode = RenderMode::try_new("zero", "one");
        assert!(mode.is_err());

        let mode = RenderMode::try_new("0", "one");
        assert!(mode.is_err());

        let mode = RenderMode::try_new("zero", "1");
        assert!(mode.is_err());

        let mode = RenderMode::try_new("", "");
        assert!(mode.is_err());

        let mode = RenderMode::try_new("0", "3");
        assert!(mode.is_ok_and(|m| m.eq(&RenderMode::Animation { start: 0, end: 3 })));
    }
}
