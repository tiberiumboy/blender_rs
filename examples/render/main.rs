use blender_rs::blend_file::BlendFile;
use blender_rs::blender::get_blend_config_from_local;
use blender_rs::blender::ComputerGraphicsProgram;
use blender_rs::blender::Frame;
use blender_rs::blender::Manager;
use blender_rs::models::event::RenderEvent;
use blender_rs::models::{args::Args, event::BlenderEvent};
use clap::Parser;
use semver::Version;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Parser)]
struct RenderCli {
    // Path to render user defined blend files
    #[arg(short, long)]
    path: Option<PathBuf>,

    // render starts from this frame
    #[arg(short, long, default_value_t = 1)]
    start: Frame,

    // end frame to render to
    #[arg(short, long, default_value_t = 5)]
    end: Frame,
}

fn render_with_manager() {
    let args = RenderCli::parse();

    let blend_path = match args.path {
        None => {
            // FIXME: Path is relative to where command is invoked. Must be from blender_rs directory, otherwise path will fail.
            let default_example_path = PathBuf::from("./examples/assets/test.blend");

            // if the default example provided with this program is missing, panic. There must be an example provided to run.
            if !default_example_path.exists() {
                let path = default_example_path.to_string_lossy();
                panic!("Blend File not found! {path}");
            }

            default_example_path
        }
        // returns if blend file location is valid.
        Some(p) if p.exists() && p.is_file() => p,
        // User provided invalid location.
        Some(invalid) => {
            let path = invalid.to_string_lossy();
            panic!("Unable to find \"{path}\"");
        }
    };

    // loads blender file and retrieve some information to display for job queue.
    let blend_file = BlendFile::try_from(&blend_path).expect("Must be a valid blend file!");

    let config = get_blend_config_from_local().expect("Unable to get blend config!");

    // Get latest blender installed, or install latest blender from web.
    let manager = Manager::load(config).expect("Must be able to launch manager to get blender");

    // Retrieve last blender version opened/used. Only contains major and minor, no patch. Rely on latest patch if possible.
    let (max, min) = blend_file.get_partial_version();

    // Minimum version required to run this blender file
    let version = Version::new(max as u64, min as u64, 0);

    // Fetch latest local version that meets the requirement version. We will not try to install,
    // so we will stop here and ask the user to load blender into configuration initially.
    let config = manager.get_config();
    let blender = config
        .get_latest_blender_available(&version)
        .expect("No local blender installation found! Must have at least one blender installed!");
    println!("Prepare blender configuration...");

    // Here we ask for the output path, for now we set our path in the same directory as our executable path.
    // This information will be display after render has been completed successfully.
    // TODO: BUG! This will save to root of C:/ on windows platform! Need to change this to current working dir
    // Do not have a copy of windows to resolve this path convention.
    let output = fs::canonicalize(PathBuf::from("./examples/assets/"))
        .expect("Must be able to collapse to absolute path!");

    // Create blender argument
    let args = Args::new(blend_file, output, args.start, args.end);

    // render the frame. Completed render will return the path of the rendered frame, error indicates failure to render due to blender incompatible hardware settings or configurations. (CPU vs GPU / Metal vs OpenGL)
    let mut listener = blender.render(args).expect(
        "Must be able to render! Please see the error and resolve the issue you may encounter",
    );

    // Handle blender status
    while let Some(status) = listener.read() {
        match status {
            BlenderEvent::Rendering(render_event) => match render_event {
                RenderEvent::Progress {
                    frame,
                    current,
                    total,
                } => {
                    let percent = (current / total) * 100.0;
                    println!("[Rendering] frame: {frame} | {current} out of {total} (%{percent})");
                }
                RenderEvent::Complete { frame, path } => {
                    println!("[Completed] frame: {frame} | path: {path:?}");
                }
            },
            BlenderEvent::Error(e) => {
                println!("[ERR] {e}");
            }
            BlenderEvent::Warning(msg) => {
                println!("[WARN] {msg}");
            }
            BlenderEvent::Info(msg) => {
                println!("[LOG] {msg}")
            }
            BlenderEvent::Busy => {
                println!("Busy...");
                continue;
            }
            _ => {
                println!("Unhandled blender status! {:?}", status);
                // break;
            }
        }
    }

    println!("Blender completed");
}

fn main() {
    render_with_manager();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assure_test_example_success() {
        render_with_manager();
    }
}
