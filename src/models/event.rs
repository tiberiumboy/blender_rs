use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BlenderEvent {
    Info(String),
    Rendering(RenderEvent),
    Warning(String),
    Exit,
    Error(String),
    Busy,
    Unhandled(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RenderEvent {
    Progress {
        frame: i32,
        current: f32,
        total: f32,
    },
    Complete {
        frame: i32,
        path: PathBuf,
    },
}
