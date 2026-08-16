use std::{
    io::{BufRead, BufReader},
    path::PathBuf,
    process::ChildStdout,
};

use crate::{
    blender::Frame,
    models::event::{BlenderEvent, RenderEvent},
};

#[derive(Debug)]
pub struct BlenderProcess {
    inner: BufReader<ChildStdout>,
    current_frame: Frame,
}

impl BlenderProcess {
    pub(crate) fn new(inner: BufReader<ChildStdout>, start_frame: Frame) -> BlenderProcess {
        BlenderProcess {
            inner,
            current_frame: start_frame,
        }
    }

    fn child_stream_to_event(&mut self, line: String) -> BlenderEvent {
        match line {
            // TODO: find a more elegant way to parse the string std out and handle invocation action.
            line if line.contains("Fra:") => {
                let col = line.split('|').collect::<Vec<&str>>();

                // this seems a bit expensive?
                let init = col[0].split(" ").next();
                if let Some(value) = init {
                    self.current_frame = value
                        .replace("Fra:", "")
                        .parse()
                        .unwrap_or(self.current_frame);
                }
                let last = col.last().unwrap().trim();
                let slice = last.split(' ').collect::<Vec<&str>>();
                match slice[0] {
                    "Rendering" => {
                        let current = slice[1].parse::<f32>().unwrap();
                        let total = slice[3].parse::<f32>().unwrap();
                        let event = RenderEvent::Progress {
                            frame: self.current_frame,
                            current,
                            total,
                        };
                        BlenderEvent::Rendering(event)
                    }
                    _ => BlenderEvent::Unhandled(line),
                }
            }

            // Do I need to care about the time?
            line if line.starts_with("Time:") => BlenderEvent::Info(line),
            line if line.contains("Use:") => BlenderEvent::Info(line),
            line if line.contains("Saved:") => {
                let location = line.split('\'').collect::<Vec<&str>>();
                let event = RenderEvent::Complete {
                    frame: self.current_frame,
                    path: PathBuf::from(location[1]),
                };
                BlenderEvent::Rendering(event)
            }

            // Strange how this was thrown, but doesn't report back to this program?
            // [ERR] Error: Engine 'BLENDER_EEVEE_NEXT' not available for scene 'Scene' (an add-on may need to be installed or enabled)
            line if line.starts_with("EXCEPTION:") => BlenderEvent::Error(line.to_owned()),
            // When launch blender for the first time, it prints out the version number and the hash information about the build)
            line if line.starts_with("Blender ") => {
                // if the line reads "Blender quit", we should send BlenderEvent::Exit signal
                if line.eq_ignore_ascii_case("blender quit") {
                    BlenderEvent::Exit
                } else {
                    BlenderEvent::Info(line)
                }
            }

            // Blender prints out reading blender files, here we'll just log the info anyway (We already have the information)
            line if line.starts_with("Read blend: ") => BlenderEvent::Info(line),

            line if line.starts_with("regiondata free error") => BlenderEvent::Warning(line),

            line if line.starts_with("Color management: ") => BlenderEvent::Info(line),

            // TODO: Warning keyword is used multiple of times. Consider removing warning apart and submit remaining content above
            line if line.contains("Warning:") => BlenderEvent::Warning(line.to_owned()),

            line if line.contains("Error:") => BlenderEvent::Error(line.to_owned()),

            line if line.eq("\n") => BlenderEvent::Busy,

            // any unhandle handler is submitted raw in console output here.
            line => BlenderEvent::Unhandled(line),
        }
    }

    pub fn read(&mut self) -> Option<BlenderEvent> {
        let mut line = String::new();

        match self.inner.read_line(&mut line) {
            Ok(len) => match len {
                // should this be busy? or block?
                // I wanted to be able to skip this line and continue, but avoid calling loop on itself?
                0 => None,
                _ => Some(self.child_stream_to_event(line)),
            },
            Err(e) => {
                eprintln!("Unable to process line! {e}");
                return None;
            }
        }
    }
}
