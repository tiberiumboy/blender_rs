#![cfg(not(doctest))]
/*
Developer blog:

Spending time on replacing xml-rpc-rs due to maintainers not willing to replace rouille plugin that supports this implementations.
I would instead incorporate the functionality of XML-RPC protocol myself instead of relying third party packages.
Reading the wikipedia - https://en.wikipedia.org/wiki/XML-RPC#Usage - xml-rpc is done via simple http server.

Currently, there is no error handling situation from blender side of things. If blender crash, we will resume the rest of the code in attempt to parse the data.
    This will eventually lead to a program crash because we couldn't parse the information we expect from stdout.
    TODO: How can I stream this data better?

- As of Blender 4.2 - they introduced BLENDER_EEVEE_NEXT as a replacement to BLENDER_EEVEE.
    Will need to make sure I pass in the correct enum for version 4.2 and above.

- Spoke to Sheepit - another "Intranet" distribution render service (Closed source)
    - In order to get Render preview window, there needs to be a GPU context to attach to.
        Otherwise, we'll have to wait for the render to complete the process before sending the image back to the user.
    - They mention to enforce compute methods, do not mix cpu and gpu. (Why?)

Advantage:
- can support M-series ARM processor.
- Original tool Doesn't composite video for you - We can make ffmpeg wrapper? - This will be a feature but not in this level of implementation.
- LogicReinc uses JSON to load batch file - difficult to adjust frame(s) after job sent.
    I'm creating an IPC between this program and python to ask next frame. To improve actions over blender.

Disadvantage:
- Currently rely on python script to do custom render within blender.
    No interops/additional cli commands other than interops through bpy (blender python) package
    Instead of using JSON to send configuration to python/blender, we're using IPC to control next frame/batch to render(s).
    Currently using Command::Process to invoke commands to blender. Would like to see if there's public API or .dll to interface into.

Challenges:
    Blender support tileX/Y, but gluing the image together is a new challenge - a 64K 24bits image would consume about 3Gb, and size exponentially grow from there.
    Have a look into NIP2 to stitch large images together - https://github.com/libvips/nip2
        TODO: Find a way to glue image async by image to image, buffer to buffer, flush out each image before loading new image and hold nothing in memory, store it all on disk instead.

WARN:
    From LogicReinc FAQ's:
        Q: Render fails due to Gdip
        A: You're running Linux or Mac but did not install libgdiplus and libc6-dev,
            install these and you should be good.

        Q:Render fails on Linux
        A:You may not have the required blender system dependencies. Easiest way to cover them all is to just run `apt-get install blender` to fetch them all.
            (It does not have to be an up2date blender package, its just for dependencies)

    Q: My Blendfile requires special addons to be active while rendering, can I add these?
    A: Blendfarm has its own versions of Blender in the BlenderData directory, and it runs
        these versions always in factory startup, thus without any added addons. This is done
        on purpose to make sure the environment is not altered. Most addons don't have to be
        active during rendering as they generate geometry etc. If you really need this, make
        an issue and I see what I can do. However do realise that this may make the workflow
        less smooth. (As you may need to set up these plugins for every Blender version instead
        of just letting BlendFarm do all the work.
    */

use crate::blender_process::BlenderProcess;
pub use crate::manager::{Manager, ManagerError};
pub use crate::models::{args::Args, blender_config::BlenderConfig};
pub use crate::utils::get_blend_config_from_local;
use crate::utils::get_config_folder_path;

use regex::Regex;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::hash::{DefaultHasher, Hasher};
use std::path::{Path, PathBuf};
use std::process::{ChildStdout, Command, Stdio};
use std::sync::LazyLock;
use std::{fmt::Display, fs, io::BufReader, num::ParseIntError};

pub type Frame = i32;

// TODO: this is ugly, and I want to get rid of this. How can I improve this?
// Backstory: Win and linux can be invoked via their direct app link. However, MacOS .app is just a bundle, which contains the executable inside.
// To run process::Command, I must properly reference the executable path inside the blender.app on MacOS, using the hardcoded path below.
#[cfg(target_os = "macos")]
pub(crate) const MACOS_PATH: &str = "Contents/MacOS/Blender";

#[derive(Debug)]
pub enum BlenderError {
    ExecutableInvalid,
    ExecutableNotFound(PathBuf),
    InvalidFile(String),
    RenderError(String),
    PythonError(String),
    ServiceOffline,
    ParseInt(ParseIntError),
    // Scary, find out if there's a better way to handle this?
    IoError(std::io::Error),
}

impl Display for BlenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BlenderError::ExecutableInvalid => f.write_str("Executable invalid"),
            BlenderError::ExecutableNotFound(path_buf) => {
                f.write_str(&format!("Executable not found at {:?}", path_buf))
            }
            BlenderError::InvalidFile(file_name) => {
                f.write_str(&format!("Invalid file: {file_name}"))
            }
            BlenderError::RenderError(message) => f.write_str(&format!("Render error: {message}")),
            BlenderError::PythonError(message) => f.write_str(&format!("Python error: {message}")),
            BlenderError::ServiceOffline => f.write_str(&format!("Service offline")),
            BlenderError::ParseInt(parse_int_error) => f.write_str(&parse_int_error.to_string()),
            BlenderError::IoError(io_error) => f.write_str(&io_error.to_string()),
        }
    }
}

// TODO: I want to avoid making this trait public as this is only used for Blender.
// Making this as a public trait expose opportunity for developers to implement their custom computer graphic programs instead of Blender as intended.
// The only reason for this trait was to use for unit testing, providing code coverage for this library usage.
// Otherwise, a Mock struct would have to be created to mimic the original object solely for unit testing.
pub trait ComputerGraphicsProgram {
    fn get_relative_path(&self) -> &Path;
    // TODO: extract args as trait
    // TODO: convert BlenderProcess into trait
    fn render(&self, args: Args) -> Result<BlenderProcess, BlenderError>;
    // TODO: Return trait type instead
    // fn from_executable<CG: ComputerGraphicsProgram>(executable: impl AsRef<Path>) -> Result<CG, IoError>
    fn from_executable(executable: impl AsRef<Path>) -> Result<Blender, BlenderError>;
    fn get_executable(&self) -> &Path;
    fn get_version(&self) -> &Version;
}

// [Note] In the sense of PartialOrd, Ord - Blender's executable would not matter if the version is identical.
/// Blender structure is to hold path to executable and version of blender installed.
/// This is the wrapper to interface with the actual blender program.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Blender {
    /// Path to blender executable on the system.
    executable: PathBuf,
    /// Version of blender installed on the system.
    version: Version,
}

// Overload to omit path ordering. Order by Version instead.
impl PartialOrd for Blender {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.version.partial_cmp(&other.version)
    }
}

// Overload to omit path ordering. Order by Version instead.
impl Ord for Blender {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.version.cmp(&other.version)
    }
}

/* Private method impl */
impl Blender {
    /// Create a new blender struct with provided path and version. This does not checked and enforced!
    ///
    /// # Examples
    /// ```
    /// use blender::Blender;
    /// let blender = Blender::new(PathBuf::from("path/to/blender"), Version::new(4,1,0));
    /// ```
    fn new(executable: impl AsRef<Path>, version: Version) -> Self {
        Self {
            executable: executable.as_ref().to_path_buf(),
            version,
        }
    }

    #[inline]
    fn handle_parse(value: &str) -> Result<u64, BlenderError> {
        value.parse().map_err(BlenderError::ParseInt)
    }

    // Adding rules to provide valid version schema for Blender software and this product controls
    pub(crate) fn parse_partial_version(
        major: &str,
        minor: &str,
        patch: Option<&str>,
    ) -> Option<Version> {
        // *filter out any major version 3 or below. We will not be supporting legacy blender at the moment.
        let major: u64 = match major.parse() {
            Ok(v) if v >= 3 => v,
            Ok(_) => return None,
            Err(e) => {
                eprintln!("{e:?}");
                return None;
            }
        };

        let minor: u64 = match minor.parse() {
            Ok(v) => v,
            Err(e) => {
                eprintln!("{e:?}");
                return None;
            }
        };

        let mut patch_number: u64 = 0;

        if let Some(p) = patch {
            patch_number = match p.parse() {
                Ok(number) => number,
                Err(_) => return None,
            }
        }

        Some(Version::new(major, minor, patch_number))
    }

    /// Obtain the version by invoking version command to blender directly.
    /// This function will invoke the -v command to retrieve blender version information.
    /// This validate two things,
    /// 1: Blender's internal version is reliable
    /// 2: Executable is functional and operational
    /// Otherwise, return an error that we were unable to verify this custom blender integrity.
    ///
    /// # Errors
    /// * InvalidData - executable path do not exist or is invalid. Please verify that the path provided exist and not compressed.
    ///  This error also serves where the executable is unable to provide the blender version.
    fn check_version(executable_path: impl AsRef<Path>) -> Result<Blender, BlenderError> {
        static VERSION_REGEX: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"Blender (?<major>[0-9]).(?<minor>[0-9]).(?<patch>[0-9])").unwrap()
        });

        // check and verify that the executable exist.
        // first line for validating blender executable.
        let exec_path = executable_path.as_ref();

        // path must exist!
        if !exec_path.exists() {
            return Err(BlenderError::ExecutableNotFound(exec_path.into()));
        }

        // macOS is special. To invoke the blender application, I need to navigate inside Blender.app, which is an app bundle that contains stuff to run blender.
        // Command::Process needs to access the content inside app bundle to perform the operation correctly.
        // To do this - I need to append additional path args to correctly invoke the right application for this to work.
        #[cfg(target_os = "macos")]
        let exec_path = if !&exec_path.ends_with(MACOS_PATH) {
            &exec_path.join(MACOS_PATH)
        } else {
            exec_path
        };

        let output = Command::new(exec_path)
            .arg("-v")
            .output()
            .map_err(BlenderError::IoError)?;

        // TODO: remove expect()
        let stdout = String::from_utf8(output.stdout)
            .expect("Should be able to read string content from this program!");
        match VERSION_REGEX.captures(&stdout) {
            Some(cap) => {
                let (_, [major, minor, patch]) = cap.extract();
                let major = Self::handle_parse(major)?;
                let minor = Self::handle_parse(minor)?;
                let patch = Self::handle_parse(patch)?;
                let version = Version::new(major, minor, patch);
                let blender = Blender::new(exec_path, version);
                Ok(blender)
            }
            None => {
                eprintln!("Found no regex matches! {stdout:?}");
                Err(BlenderError::ExecutableInvalid)
            }
        }
    }

    /// desire location to load blender python script file from. This is also used to verify checksums.
    fn get_script_path() -> Result<PathBuf, BlenderError> {
        Ok(get_config_folder_path()
            .map_err(BlenderError::IoError)?
            .join("render.py"))
    }

    /// Used to verify the integrity of the python file we rely on invoking the blender jobs.
    fn calculate_checksum(input: &[u8]) -> u64 {
        let mut hash = DefaultHasher::new();
        for bit in input {
            hash.write_u8(*bit);
        }
        hash.finish()
    }

    /// Invoke blender with the provided arguments.
    fn invoke(&self, args: Args) -> Result<BufReader<ChildStdout>, BlenderError> {
        let script_path = Self::get_script_path()?;
        let data = include_bytes!("./render.py");
        // design to ensure the python script is up to date and matches with this BlendFarm internal script version.
        // This is to prevent unauthorized script changes made by clients.
        if !script_path.exists() {
            fs::write(&script_path, data).map_err(BlenderError::IoError)?;
        } else {
            let content = fs::read(&script_path).map_err(BlenderError::IoError)?;
            let source = Self::calculate_checksum(data);
            let target = Self::calculate_checksum(&content);
            if source != target {
                fs::write(&script_path, data).map_err(BlenderError::IoError)?;
            }
        }

        let col = &args.generate_arg_command(script_path)?;
        let stdout = Command::new(&self.executable)
            .args(col)
            .stdout(Stdio::piped())
            .spawn()
            .map_err(BlenderError::IoError)?
            .stdout
            .ok_or(BlenderError::RenderError(
                "Unable to retrieve std output!".to_owned(),
            ))?;

        Ok(BufReader::new(stdout))
    }
}

impl ComputerGraphicsProgram for Blender {
    // the difference between this function and getting executable are
    // a) MacOs is special. Executable reference a path inside app bundle.
    // b) This returns valid dir location to open to for user to look at from file POV
    // TODO: Remove all of this unwrap nightmare.
    fn get_relative_path(&self) -> &Path {
        if cfg!(target_os = "macos") {
            &self
                .executable
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .parent()
                .unwrap()
        } else {
            &self.executable.parent().unwrap()
        }
    }

    /// Return the executable path to blender (Entry point for CLI)
    fn get_executable(&self) -> &Path {
        &self.executable
    }

    /// Return validated Blender Version
    fn get_version(&self) -> &Version {
        &self.version
    }

    /// Create a new blender struct from executable path. This function will fetch the version of blender by invoking -v command.
    /// Otherwise, if Blender is not install, or a version is not found, an error will throw
    ///
    /// # Error
    ///
    /// * InvalidData - executable path do not exist, or is invalid. Please verify that the executable path is correct and leads to the actual executable.
    /// *
    /// # Examples
    /// ```
    /// use blender::Blender;
    /// let blender = ComputerGraphicsProgram<Blender>::from_executable(Pathbuf::from("../examples/")).unwrap();
    /// ```
    fn from_executable(executable: impl AsRef<Path>) -> Result<Blender, BlenderError> {
        Self::check_version(executable)
    }

    /// Render one frame - can we make the assumption that ProjectFile may have configuration predefined Or is that just a system global setting to apply on?
    /// # Examples
    /// ```
    /// use blender::Blender;
    /// use blender::args::Args;
    /// let blender = Blender::from_executable("path/to/blender").unwrap();
    /// let args = Args::new(PathBuf::from("path/to/project.blend"), PathBuf::from("path/to/output.png"));
    /// let final_output = blender.render(&args).unwrap();
    /// ```
    // so instead of just returning the string of render result or blender error, we'll simply use the single producer to produce result from this class.
    // issue here is that we need to lock thread. If we are rendering, we need to be able to call abort.
    fn render(&self, args: Args) -> Result<BlenderProcess, BlenderError> {
        let start_frame = args.start;
        // I received a No such file or directory error here?
        let child_proc = self.invoke(args)?;
        Ok(BlenderProcess::new(child_proc, start_frame))
    }
}

#[cfg(test)]
pub(crate) mod test {
    use super::*;
    #[cfg(target_os = "macos")]
    use crate::blender::MACOS_PATH;
    use blend::Instance;

    // must be accessible within crate for unit test purposes.
    pub(crate) fn mock_blender(path: Option<PathBuf>, version: Version) -> Blender {
        match path {
            Some(executable) => Blender {
                executable,
                version,
            },
            None => Blender {
                executable: PathBuf::new(),
                version,
            },
        }
    }

    // this is used to read and preview raw values from blendfile in cli friendly view mode
    #[allow(dead_code)]
    fn explore_value<'a>(obj: &Instance<'a>) {
        use blend::parsers::field::FieldInfo;

        for i in &obj.fields {
            match i.1.is_primitive {
                true => match i.1.info {
                    FieldInfo::Value => {
                        match i.1.type_name.as_str() {
                            "int" => {
                                println!("{}: {} = {} ", i.0, i.1.type_name, &obj.get_i32(i.0));
                            }
                            "short" => {
                                println!("{}: {} = {} ", i.0, i.1.type_name, &obj.get_u16(i.0));
                            }
                            "char" => {
                                println!("{}: {} = {} ", i.0, i.1.type_name, &obj.get_string(i.0));
                            }
                            "float" => {
                                println!("{}: {} = {}", i.0, i.1.type_name, &obj.get_f32(i.0));
                            }
                            "uint64_t" => {
                                println!("{}: {} = {}", i.0, i.1.type_name, &obj.get_u64(i.0));
                            }
                            _ => println!("Unhandle value for {} | {}", i.1.type_name, i.0),
                        };
                    }
                    FieldInfo::ValueArray { .. } => match i.1.type_name.as_str() {
                        "char" => {
                            println!("{}: String = {}", i.0, &obj.get_string(i.0));
                        }
                        "float" => {
                            println!("{}: vec<f32> = {:?}", i.0, &obj.get_f32_vec(i.0));
                        }
                        _ => {
                            println!("Unhandle Value Array for {} | {}", i.1.type_name, i.0)
                        }
                    },
                    _ => {
                        println!("Unhandle: {} | {} ", i.0, i.1.type_name)
                    }
                },
                false => {
                    println!("{}: TYPE = {}", i.0, i.1.type_name);
                }
            }
        }
    }

    #[test]
    fn assure_blender_match_version_succeed() {
        // https://download.blender.org/release/Blender4.0/
        let lvalue = mock_blender(None, Version::new(4, 0, 1));
        let rvalue = mock_blender(None, Version::new(4, 0, 1));
        assert!(&lvalue.eq(&rvalue));

        // older version, lvalue should be greater.
        let rvalue = mock_blender(None, Version::new(3, 6, 9));
        assert!(&lvalue.gt(&rvalue));

        // newer patch, lvalue should be less than.
        let rvalue = mock_blender(None, Version::new(4, 0, 2));
        assert!(&lvalue.lt(&rvalue));
    }

    #[test]
    fn assure_get_script_path_succeed() {
        let path = Blender::get_script_path();
        assert!(path.is_ok_and(|path| path.exists() && path.is_file()));
    }

    #[test]
    fn assure_order_works() {
        let newer = Blender::new(PathBuf::new(), Version::new(4, 3, 4));
        let older = Blender::new(PathBuf::new(), Version::new(4, 2, 4));
        let mut list = vec![newer.clone(), older.clone()];
        list.sort();
        assert_eq!(list[0], older);
        assert_eq!(list[1], newer);
    }

    #[test]
    fn assure_handle_parse_succeed() {
        let expect = 4;
        let value = expect.to_string();
        let parse = Blender::handle_parse(&value);
        assert!(parse.is_ok_and(|f| f.eq(&expect)));

        let value = "A";
        let parse = Blender::handle_parse(value);
        assert!(parse.is_err());
    }

    #[test]
    fn assure_display_blender_error_succeed() {
        assert_eq!(
            "Service offline",
            format!("{}", BlenderError::ServiceOffline)
        );
        // test invalid executable
        assert_eq!(
            "Executable invalid",
            format!("{}", BlenderError::ExecutableInvalid)
        );
        let path = PathBuf::new();
        assert_eq!(
            format!("Executable not found at {:?}", path),
            format!("{}", BlenderError::ExecutableNotFound(path))
        );

        let file_name = "test.txt";
        assert_eq!(
            format!("Invalid file: {file_name}"),
            format!("{}", BlenderError::InvalidFile(file_name.to_owned()))
        );

        let parse_int_error: ParseIntError = "a".parse::<i32>().expect_err("Should fail to parse");
        assert_eq!(
            parse_int_error.to_string(),
            format!("{}", BlenderError::ParseInt(parse_int_error))
        );

        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "Not found - unit test");
        assert_eq!(
            io_error.to_string(),
            format!("{}", BlenderError::IoError(io_error))
        );

        let message = "";
        assert_eq!(
            format!("Render error: {message}"),
            format!("{}", BlenderError::RenderError(message.to_owned()))
        );
        assert_eq!(
            format!("Python error: {message}"),
            format!("{}", BlenderError::PythonError(message.to_owned()))
        );
    }

    #[test]
    fn assure_check_sum_succeed() {
        let a = "test";
        let b = "t3st";
        let a_value = Blender::calculate_checksum(a.as_bytes());
        let b_value = Blender::calculate_checksum(b.as_bytes());
        assert_ne!(a_value, b_value);

        assert_eq!(a_value, Blender::calculate_checksum(a.as_bytes()));
        assert_eq!(a_value, 16183295663280961421);

        assert_eq!(b_value, Blender::calculate_checksum(b.as_bytes()));
        assert_eq!(b_value, 10932941976625646637);
    }

    #[cfg(target_os = "macos")]
    fn generate_executable_path() -> PathBuf {
        PathBuf::from("./blender4.0/blender").join(MACOS_PATH)
    }

    #[cfg(not(target_os = "macos"))]
    fn generate_executable_path() -> PathBuf {
        PathBuf::from("./blender4.0/blender")
    }

    #[test]
    fn assure_get_relative_path_succeed() {
        let executable = generate_executable_path();
        let version = Version::new(4, 0, 0);
        let blender = Blender::new(executable.clone(), version);

        let path = blender.get_relative_path();
        let mut expected = executable.parent().expect("Should return ./blender4.0/");
        if cfg!(target_os = "macos") {
            expected = expected
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .parent()
                .unwrap();
        }
        assert_eq!(path, expected);
    }

    #[test]
    fn assure_valid_version_response() {
        // assure successful result
        let version = Blender::parse_partial_version("4", "3", None);
        assert!(version.is_some());
        let version = Blender::parse_partial_version("3", "0", Some("0"));
        assert!(version.is_some());
        // God forbid I live this long to see this version sematics.
        let version = Blender::parse_partial_version("999", "999", Some("999"));
        assert!(version.is_some());

        // if the value is below than 3 - return none, as we do not support blender 3.0 versions and below
        let version = Blender::parse_partial_version("2", "0", None);
        assert!(version.is_none());
        let version = Blender::parse_partial_version("1", "0", None);
        assert!(version.is_none());

        let version = Blender::parse_partial_version("4", "A", None);
        assert!(version.is_none());

        let version = Blender::parse_partial_version("4", "0", Some("B"));
        assert!(version.is_none());
    }
}
