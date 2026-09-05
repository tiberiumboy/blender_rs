use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
// TODO: Find a way to convert enum into String literal for json de/serialize
pub enum Processor {
    NONE,
    CUDA,
    OPTIX,
    HIP,
    ONEAPI,
    // is there METAL?
}
