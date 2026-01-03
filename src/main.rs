use std::env;
use std::error::Error;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use glob::glob;

// -------------------------------
// CLI Enums
// -------------------------------

#[derive(Debug, Clone, Copy)]
enum VisualizationType {
    Waveform,
    Spectrum,
    Both,
}

impl std::str::FromStr for VisualizationType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "wave" | "waveform" => Ok(VisualizationType::Waveform),
            "spectrum" | "spec" => Ok(VisualizationType::Spectrum),
            "both" => Ok(VisualizationType::Both),
            _ => Err(format!(
                "Unknown visualization type: {}. Use 'wave', 'spectrum', or 'both'.",
                s
            )),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum SpectrumColorScheme {
    Rainbow,
    Moreland,
    Nebulae,
    Fire,
    Fiery,
    Fruit,
    Cool,
    Magma,
    Green,
    Viridis,
    Plasma,
    Cividis,
    Terrain,
}

impl std::str::FromStr for SpectrumColorScheme {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "rainbow" => Ok(SpectrumColorScheme::Rainbow),
            "moreland" => Ok(SpectrumColorScheme::Moreland),
            "nebulae" => Ok(SpectrumColorScheme::Nebulae),
            "fire" => Ok(SpectrumColorScheme::Fire),
            "fiery" => Ok(SpectrumColorScheme::Fiery),
            "fruit" => Ok(SpectrumColorScheme::Fruit),
            "cool" => Ok(SpectrumColorScheme::Cool),
            "magma" => Ok(SpectrumColorScheme::Magma),
            "green" => Ok(SpectrumColorScheme::Green),
            "viridis" => Ok(SpectrumColorScheme::Viridis),
            "plasma" => Ok(SpectrumColorScheme::Plasma),
            "cividis" => Ok(SpectrumColorScheme::Cividis),
            "terrain" => Ok(SpectrumColorScheme::Terrain),
            _ => Err(format!("Unknown color scheme: {}. Use 'rainbow', 'moreland', 'nebulae', 'fire', 'fiery', 'fruit', 'cool', 'magma', 'green', 'viridis', 'plasma', 'cividis', 'terrain'", s))
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum VisualizationPosition {
    Top,
    Bottom,
    Left,
    Right,
    Center,
    Custom(u32, u32),
}

impl std::str::FromStr for VisualizationPosition {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "top" => Ok(VisualizationPosition::Top),
            "bottom" => Ok(VisualizationPosition::Bottom),
            "left" => Ok(VisualizationPosition::Left),
            "right" => Ok(VisualizationPosition::Right),
            "center" => Ok(VisualizationPosition::Center),
            _ if s.starts_with("xy(") && s.ends_with(')') => {
                let coords: Vec<&str> = s.trim_matches(|c| c == 'x' || c == 'y' || c == '(' || c == ')')
                    .split(',')
                    .collect();
                if coords.len() == 2 {
                    let x = coords[0].trim().parse().map_err(|_| "Invalid x coordinate")?;
                    let y = coords[1].trim().parse().map_err(|_| "Invalid y coordinate")?;
                    Ok(VisualizationPosition::Custom(x, y))
                } else {
                    Err("Invalid position format. Use 'xy(x,y)'".into())
                }
            },
            _ => Err(format!("Unknown position: {}. Use 'top', 'bottom', 'left', 'right', 'center', or 'xy(x,y)'", s))
        }
    }
}

// -------------------------------
// Config
// -------------------------------

#[derive(Debug, Clone)]
struct VideoConfig {
    image_path: Option<String>, // optional
    audio_path: String,
    output_path: String,
    viz_type: VisualizationType,
    duration: Option<f32>,
    position: VisualizationPosition,
    color_scheme: SpectrumColorScheme,
    width: u32,
    height: u32,
    margin: u32,
    verbose: bool,

    // Cover extraction controls
    cover_from_audio: bool,
    cover_out: Option<String>, // only honored when processing a single file
}

impl Default for VideoConfig {
    fn default() -> Self {
        Self {
            image_path: None,
            audio_path: String::new(),
            output_path: String::new(),
            viz_type: VisualizationType::Waveform, // default changed to Wave
            duration: None,
            position: VisualizationPosition::Bottom,
            color_scheme: SpectrumColorScheme::Viridis,
            width: 1280,
            height: 180,
            margin: 50,
            verbose: false,

            cover_from_audio: false,
            cover_out: None,
        }
    }
}

#[derive(Debug, Clone)]
struct AppConfig {
    // multiple inputs supported (expanded from glob)
    inputs: Vec<String>,
    out_dir: Option<String>, // if set, write outputs here
    // shared options for all
    shared: SharedOpts,
}

#[derive(Debug, Clone)]
struct SharedOpts {
    image_path: Option<String>,
    viz_type: VisualizationType,
    duration: Option<f32>,
    position: VisualizationPosition,
    color_scheme: SpectrumColorScheme,
    width: u32,
    height: u32,
    margin: u32,
    verbose: bool,
    cover_from_audio: bool,
    cover_out: Option<String>, // ignored when batch
}

impl Default for SharedOpts {
    fn default() -> Self {
        Self {
            image_path: None,
            viz_type: VisualizationType::Waveform, // default changed to Wave
            duration: None,
            position: VisualizationPosition::Bottom,
            color_scheme: SpectrumColorScheme::Viridis,
            width: 1280,
            height: 180,
            margin: 50,
            verbose: false,
            cover_from_audio: false,
            cover_out: None,
        }
    }
}

fn print_usage() {
    println!("Usage: mp3tomp4 <audio_file_or_glob> [options]");
    println!("\nExamples:");
    println!("  mp3tomp4 song.mp3                         # writes song.mp4 next to song.mp3");
    println!("  mp3tomp4 \"*.mp3\"                         # batch converts all MP3s in cwd");
    println!("  mp3tomp4 music/*.mp3 --out-dir out/       # batch to a different directory");
    println!("  mp3tomp4 track.mp3 --image cover.jpg      # explicit image");
    println!("  mp3tomp4 track.mp3 --cover-from-audio     # force embedded art");
    println!("\nOptions:");
    println!("  --image <path>        Optional explicit background image");
    println!(
        "  --cover-from-audio    Ignore --image and extract embedded cover art from the audio"
    );
    println!("  --cover-out <path>    Also save the extracted cover image (single input only)");
    println!("  --out-dir <dir>       Write outputs to this directory (filenames still derived)");
    println!("  --type <type>         'wave' (default), 'spectrum', or 'both'");
    println!("  --duration <sec>      Max duration seconds (optional)");
    println!("  --position <pos>      'top' | 'bottom' | 'left' | 'right' | 'center' | 'xy(x,y)' (default: bottom)");
    println!("  --color <scheme>      'rainbow'|'moreland'|'nebulae'|'fire'|'fiery'|'fruit'|'cool'|'magma'|'green'|'viridis'|'plasma'|'cividis'|'terrain'");
    println!("  --width <px>          Viz width (default 1280)");
    println!("  --height <px>         Viz height (default 180)");
    println!("  --margin <px>         Margin (default 50)");
    println!("  --verbose             Show ffmpeg output");
    println!();
}

fn parse_args() -> Result<Option<AppConfig>, Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    parse_args_from(args)
}

fn parse_args_from(args: Vec<String>) -> Result<Option<AppConfig>, Box<dyn Error>> {
    if args.len() < 2 {
        print_usage();
        return Ok(None);
    }

    let mut inputs: Vec<String> = Vec::new();
    let glob_or_file = args[1].clone();

    // parse options
    let mut shared = SharedOpts::default();
    let mut out_dir: Option<String> = None;

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--image" => {
                i += 1;
                if i < args.len() {
                    shared.image_path = Some(args[i].clone());
                } else {
                    return Err("--image requires a path".into());
                }
            }
            "--cover-from-audio" => {
                shared.cover_from_audio = true;
            }
            "--cover-out" => {
                i += 1;
                if i < args.len() {
                    shared.cover_out = Some(args[i].clone());
                } else {
                    return Err("--cover-out requires a path".into());
                }
            }
            "--out-dir" => {
                i += 1;
                if i < args.len() {
                    out_dir = Some(args[i].clone());
                } else {
                    return Err("--out-dir requires a directory path".into());
                }
            }
            "--type" => {
                i += 1;
                if i < args.len() {
                    shared.viz_type = args[i].parse()?;
                }
            }
            "--duration" => {
                i += 1;
                if i < args.len() {
                    shared.duration = Some(args[i].parse()?);
                }
            }
            "--position" => {
                i += 1;
                if i < args.len() {
                    shared.position = args[i].parse()?;
                }
            }
            "--color" => {
                i += 1;
                if i < args.len() {
                    shared.color_scheme = args[i].parse()?;
                }
            }
            "--width" => {
                i += 1;
                if i < args.len() {
                    shared.width = args[i].parse()?;
                }
            }
            "--height" => {
                i += 1;
                if i < args.len() {
                    shared.height = args[i].parse()?;
                }
            }
            "--margin" => {
                i += 1;
                if i < args.len() {
                    shared.margin = args[i].parse()?;
                }
            }
            "--verbose" => {
                shared.verbose = true;
            }
            unknown => return Err(format!("Unknown argument: {}", unknown).into()),
        }
        i += 1;
    }

    // Expand glob; if no match, use as literal file if exists; else error
    let mut matched = false;
    for entry in glob(&glob_or_file)? {
        matched = true;
        if let Ok(path) = entry {
            if path.is_file() {
                inputs.push(path.to_string_lossy().into_owned());
            }
        }
    }

    if !matched {
        // treat as literal path
        if Path::new(&glob_or_file).is_file() {
            inputs.push(glob_or_file);
        } else {
            return Err(format!(
                "No files matched pattern or file not found: {}",
                glob_or_file
            )
            .into());
        }
    }

    // if batch and --cover-out provided → ignore (single-file convenience)
    if inputs.len() > 1 && shared.cover_out.is_some() {
        eprintln!("Warning: --cover-out is ignored in batch mode (multiple inputs).");
        shared.cover_out = None;
    }

    Ok(Some(AppConfig {
        inputs,
        out_dir,
        shared,
    }))
}

// -------------------------------
// Spectrum helper fns
// -------------------------------

fn get_spectrum_params(
    pos: VisualizationPosition,
    width: u32,
    height: u32,
) -> (u32, u32, &'static str) {
    match pos {
        VisualizationPosition::Left | VisualizationPosition::Right => (height, width, "vertical"),
        _ => (width, height, "horizontal"),
    }
}

fn get_filter_complex(config: &VideoConfig) -> String {
    // Common background scaling
    let base = "[0:v]scale=1280:720:force_original_aspect_ratio=decrease,pad=1280:720:(ow-iw)/2:(oh-ih)/2[bg]";

    match config.viz_type {
        VisualizationType::Waveform => {
            format!(
                "{}; \
                [1:a]aformat=channel_layouts=mono,\
                showwaves=s={}x{}:mode=line:rate=25:colors=white[wave]; \
                [bg][wave]overlay={}",
                base,
                config.width,
                config.height,
                get_position_overlay(config.position, config.margin)
            )
        }
        VisualizationType::Spectrum => {
            let (spec_width, spec_height, orientation) =
                get_spectrum_params(config.position, config.width, config.height);
            let spec_params =
                get_color_args(config.color_scheme, spec_width, spec_height, orientation);

            format!(
                "{}; \
                [1:a]aformat=channel_layouts=mono,showspectrum={}[spec]; \
                [bg][spec]overlay={}",
                base,
                spec_params,
                get_position_overlay(config.position, config.margin)
            )
        }
        VisualizationType::Both => {
            let gap = config.margin / 2; // Dynamic gap based on margin
            let (wave_height, _half_spec_h) = match config.position {
                VisualizationPosition::Left | VisualizationPosition::Right => {
                    (config.width / 2, config.width / 2) // For vertical layout
                }
                _ => (config.height / 2, config.height / 2), // For horizontal layout
            };

            let (spec_width, spec_height, orientation) =
                get_spectrum_params(config.position, config.width, wave_height);
            let spec_params =
                get_color_args(config.color_scheme, spec_width, spec_height, orientation);

            let (wave_pos, spec_pos) = match config.position {
                VisualizationPosition::Bottom => (
                    format!("x=(W-w)/2:y=H-h-{}-{}", spec_height + gap, config.margin),
                    format!("x=(W-w)/2:y=H-h-{}", config.margin),
                ),
                VisualizationPosition::Top => (
                    format!("x=(W-w)/2:y={}", config.margin),
                    format!(
                        "x=(W-w)/2:y={}+{}",
                        wave_height + gap + config.margin,
                        config.margin
                    ),
                ),
                VisualizationPosition::Left => (
                    format!("x={}:y=(H-h)/2", config.margin),
                    format!(
                        "x={}+{}:y=(H-h)/2",
                        wave_height + gap + config.margin,
                        config.margin
                    ),
                ),
                VisualizationPosition::Right => (
                    format!("x=W-w-{}-{}:y=(H-h)/2", spec_width + gap, config.margin),
                    format!("x=W-w-{}:y=(H-h)/2", config.margin),
                ),
                VisualizationPosition::Center => (
                    format!("x=(W-w)/2:y=(H-h)/2-{}", wave_height / 2 + gap / 2),
                    format!("x=(W-w)/2:y=(H-h)/2+{}", gap / 2),
                ),
                VisualizationPosition::Custom(x, y) => (
                    format!("x={}:y={}", x, y),
                    format!("x={}:y={}+{}", x, y + wave_height, gap),
                ),
            };

            format!(
                "{}; \
                [1:a]aformat=channel_layouts=mono,showwaves=s={}x{}:mode=line:rate=25:colors=white[wave]; \
                [1:a]aformat=channel_layouts=mono,showspectrum={}[spec]; \
                [bg][wave]overlay={}[tmp]; \
                [tmp][spec]overlay={}",
                base,
                config.width, wave_height,
                spec_params,
                wave_pos,
                spec_pos
            )
        }
    }
}

// Updated color args function to handle orientation
fn get_color_args(
    scheme: SpectrumColorScheme,
    width: u32,
    height: u32,
    orientation: &str,
) -> String {
    let base_args = format!(
        "s={}x{}:mode=combined:scale=cbrt:slide=scroll:fscale=lin:\
        win_func=hamming:overlap=0:fps=auto:start=100:stop=10000:orientation={}",
        width,
        height,
        if orientation == "vertical" { "1" } else { "0" }
    );

    let color = match scheme {
        SpectrumColorScheme::Rainbow => "rainbow",
        SpectrumColorScheme::Moreland => "moreland",
        SpectrumColorScheme::Nebulae => "nebulae",
        SpectrumColorScheme::Fire => "fire",
        SpectrumColorScheme::Fiery => "fiery",
        SpectrumColorScheme::Fruit => "fruit",
        SpectrumColorScheme::Cool => "cool",
        SpectrumColorScheme::Magma => "magma",
        SpectrumColorScheme::Green => "green",
        SpectrumColorScheme::Viridis => "viridis",
        SpectrumColorScheme::Plasma => "plasma",
        SpectrumColorScheme::Cividis => "cividis",
        SpectrumColorScheme::Terrain => "terrain",
    };

    format!("{}:color={}", base_args, color)
}

fn get_position_overlay(pos: VisualizationPosition, margin: u32) -> String {
    match pos {
        VisualizationPosition::Top => format!("x=(W-w)/2:y={}", margin),
        VisualizationPosition::Bottom => format!("x=(W-w)/2:y=H-h-{}", margin),
        VisualizationPosition::Left => format!("x={}:y=(H-h)/2", margin),
        VisualizationPosition::Right => format!("x=W-w-{}:y=(H-h)/2", margin),
        VisualizationPosition::Center => "x=(W-w)/2:y=(H-h)/2".to_string(),
        VisualizationPosition::Custom(x, y) => format!("x={}:y={}", x, y),
    }
}

// -------------------------------
// Cover extraction helpers
// -------------------------------

fn ext_from_mime(mime: &str) -> &'static str {
    match mime {
        "image/jpeg" | "image/jpg" => "jpg",
        "image/png" => "png",
        "image/webp" => "webp",
        _ => "bin",
    }
}

fn temp_cover_path_with_ext(ext: &str) -> Result<PathBuf, Box<dyn Error>> {
    let ts = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    Ok(env::temp_dir().join(format!("cover_{}_{}.{}", std::process::id(), ts, ext)))
}

fn extract_cover_via_id3(
    audio_path: &str,
    save_to: Option<&str>,
) -> Result<PathBuf, Box<dyn Error>> {
    let tag = id3::Tag::read_from_path(audio_path)?;
    let mut chosen = None;
    for p in tag.pictures() {
        if p.picture_type == id3::frame::PictureType::CoverFront {
            chosen = Some(p.clone());
            break;
        }
        if chosen.is_none() {
            chosen = Some(p.clone());
        }
    }
    let pic = chosen.ok_or("No embedded picture found in ID3")?;
    let ext = ext_from_mime(&pic.mime_type);
    let out = if let Some(dst) = save_to {
        PathBuf::from(dst)
    } else {
        temp_cover_path_with_ext(ext)?
    };
    std::fs::write(&out, &pic.data)?;
    Ok(out)
}

fn extract_cover_via_ffmpeg(
    audio_path: &str,
    save_to: Option<&str>,
) -> Result<PathBuf, Box<dyn Error>> {
    // Probe to guess extension. First try specifically for attached_pic (covers)
    let mut probe_args = vec![
        "-v",
        "error",
        "-select_streams",
        "v:attached_pic",
        "-show_entries",
        "stream=codec_name",
        "-of",
        "default=noprint_wrappers=1:nokey=1",
        audio_path,
    ];

    let mut probe = Command::new("ffprobe").args(&probe_args).output()?;
    let mut codec = String::from_utf8_lossy(&probe.stdout).trim().to_string();

    if codec.is_empty() {
        // Fallback to first video stream (useful for video files used as audio source)
        probe_args[3] = "v:0";
        probe = Command::new("ffprobe").args(&probe_args).output()?;
        codec = String::from_utf8_lossy(&probe.stdout).trim().to_string();
    }

    if codec.is_empty() {
        return Err("No attached picture or video stream found".into());
    }

    let ext = match codec.as_str() {
        "png" => "png",
        "webp" => "webp",
        _ => "jpg", // Default to jpg for video frames or mjpeg
    };

    let out = if let Some(dst) = save_to {
        PathBuf::from(dst)
    } else {
        temp_cover_path_with_ext(ext)?
    };

    // Extract the stream. We don't use -c copy here to ensure
    // that any video frame is properly transcoded to the target image format (e.g. h264 -> jpg).
    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-i",
            audio_path,
            "-an",
            "-map",
            "0:v:0",
            "-frames:v",
            "1", // Only one frame
            out.to_str().ok_or("Invalid cover output path")?,
        ])
        .status()?;

    if !status.success() {
        return Err("ffmpeg failed to extract attached picture or video frame".into());
    }
    Ok(out)
}

/// Attempts to extract cover art to a temp file (or user path if provided).
/// Returns the path to the extracted file.
fn extract_cover_to_file(
    audio_path: &str,
    optional_out: Option<&str>,
) -> Result<PathBuf, Box<dyn Error>> {
    match extract_cover_via_id3(audio_path, optional_out) {
        Ok(p) => Ok(p),
        Err(e1) => {
            // fallback to ffmpeg if available
            let ff_ok = Command::new("ffmpeg")
                .arg("-version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok();
            if ff_ok {
                extract_cover_via_ffmpeg(audio_path, optional_out).map_err(|e2| {
                    format!("Cover not found via ID3 ({e1}); ffmpeg fallback also failed: {e2}")
                        .into()
                })
            } else {
                Err(
                    format!("Cover not found via ID3 ({e1}) and ffmpeg not available for fallback")
                        .into(),
                )
            }
        }
    }
}

// -------------------------------
// Thumbnail helper
// -------------------------------

fn write_thumbnail(
    image_input_path: &str,
    audio_path: &str,
    output_video_path: &str,
    verbose: bool,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    use std::ffi::OsStr;

    let audio_stem = std::path::Path::new(audio_path)
        .file_stem()
        .and_then(OsStr::to_str)
        .ok_or("Invalid audio filename")?;

    // Put the thumbnail in the same dir as the .mp4 (handles --out-dir too)
    let out_dir = std::path::Path::new(output_video_path)
        .parent()
        .unwrap_or(std::path::Path::new("."));

    // Prefer PNG only if the source is a PNG; otherwise use JPG (YouTube-friendly)
    let src_ext = std::path::Path::new(image_input_path)
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or("")
        .to_ascii_lowercase();

    let want_ext = if src_ext == "png" { "png" } else { "jpg" };
    let dest = out_dir.join(format!("{}.{}", audio_stem, want_ext));

    // If we already have the right format, just copy; else transcode via ffmpeg
    if (src_ext == "jpg" || src_ext == "jpeg" || src_ext == "png") && src_ext == want_ext {
        if std::path::Path::new(image_input_path) != dest {
            std::fs::copy(image_input_path, &dest)?;
        }
    } else {
        let mut cmd = std::process::Command::new("ffmpeg");
        cmd.args(["-y", "-i", image_input_path, "-frames:v", "1"]);
        if want_ext == "jpg" {
            // good quality jpeg for thumbnails
            cmd.args(["-q:v", "2"]);
        }
        cmd.arg(dest.to_str().ok_or("Bad thumbnail output path")?);

        if !verbose {
            cmd.stdout(std::process::Stdio::null());
            cmd.stderr(std::process::Stdio::null());
        }

        let status = cmd.status()?;
        if !status.success() {
            return Err("Failed to write thumbnail via ffmpeg".into());
        }
    }

    println!("Thumbnail saved: {}", dest.display());
    Ok(dest)
}

// -------------------------------
// Video creation (uses cover if needed)
// -------------------------------

fn create_video(config: VideoConfig) -> Result<(), Box<dyn Error>> {
    // Validate audio first
    if !Path::new(&config.audio_path).exists() {
        return Err(format!("Audio file not found: {}", config.audio_path).into());
    }

    // Resolve image path
    let mut temp_cover_to_delete: Option<PathBuf> = None;
    let image_input_path: String = {
        let need_extract = config.cover_from_audio
            || config
                .image_path
                .as_ref()
                .is_none_or(|p| !Path::new(p).exists());

        if need_extract {
            let out_hint = config.cover_out.as_deref();
            let p = extract_cover_to_file(&config.audio_path, out_hint)?;
            if out_hint.is_none() {
                temp_cover_to_delete = Some(p.clone());
            }
            p.to_string_lossy().into_owned()
        } else {
            // image_path exists and we are not forcing cover-from-audio
            config.image_path.clone().unwrap()
        }
    };

    // Get audio duration
    let duration = Command::new("ffprobe")
        .arg("-v")
        .arg("error")
        .arg("-show_entries")
        .arg("format=duration")
        .arg("-of")
        .arg("default=noprint_wrappers=1:nokey=1")
        .arg(&config.audio_path)
        .output()?;

    let audio_duration: f32 = String::from_utf8_lossy(&duration.stdout)
        .trim()
        .parse()
        .unwrap_or(0.0);

    let target_duration = config.duration.unwrap_or(audio_duration);

    // Create temporary file with a unique name
    let temp_video = env::temp_dir().join(format!("temp_video_{}.mp4", std::process::id()));
    let temp_video_path = temp_video
        .to_str()
        .ok_or("Failed to create temporary path")?;

    println!("Creating temporary file at: {}", temp_video_path);

    // Generate the filter complex string
    let filter = get_filter_complex(&config);

    println!("Step 1: Creating visualization video...");

    let mut step1 = Command::new("ffmpeg");
    step1
        .arg("-y")
        .arg("-i")
        .arg(&image_input_path)
        .arg("-i")
        .arg(&config.audio_path)
        .arg("-filter_complex")
        .arg(&filter)
        .arg("-c:v")
        .arg("libx264")
        .arg("-c:a")
        .arg("aac")
        .arg("-preset")
        .arg("ultrafast")
        .arg("-tune")
        .arg("stillimage")
        .arg("-t")
        .arg(target_duration.to_string())
        .arg("-pix_fmt")
        .arg("yuv420p")
        .arg(temp_video_path);

    if !config.verbose {
        step1.stderr(Stdio::piped());
    }

    let mut step1_child = step1.spawn()?;

    if !config.verbose {
        let mut had_error = false;
        if let Some(stderr) = step1_child.stderr.take() {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                if line.contains("Error") || line.contains("error") {
                    println!("FFmpeg error: {}", line);
                    had_error = true;
                } else if line.contains("frame=") || line.contains("time=") {
                    print!("\r{}", line);
                    std::io::stdout().flush().unwrap_or(());
                }
            }
        }

        let status = step1_child.wait()?;
        if !status.success() || had_error {
            return Err("Step 1: FFmpeg visualization creation failed".into());
        }
    } else {
        let status = step1_child.wait()?;
        if !status.success() {
            return Err("Step 1: FFmpeg visualization creation failed".into());
        }
    }

    // Verify the temporary file was created
    if !Path::new(temp_video_path).exists() {
        return Err(format!("Failed to create temporary file at {}", temp_video_path).into());
    }

    println!("\nStep 2: Combining with audio...");

    let mut step2 = Command::new("ffmpeg");
    step2
        .arg("-y")
        .arg("-i")
        .arg(temp_video_path)
        .arg("-i")
        .arg(&config.audio_path)
        .arg("-map")
        .arg("0:v:0")
        .arg("-map")
        .arg("1:a:0")
        .arg("-c:v")
        .arg("copy")
        .arg("-c:a")
        .arg("aac")
        .arg("-shortest")
        .arg(&config.output_path);

    if !config.verbose {
        step2.stderr(Stdio::piped());
    }

    let mut step2_child = step2.spawn()?;

    if !config.verbose {
        let mut had_error = false;
        if let Some(stderr) = step2_child.stderr.take() {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                if line.contains("Error") || line.contains("error") {
                    println!("FFmpeg error: {}", line);
                    had_error = true;
                } else if line.contains("frame=") || line.contains("time=") {
                    print!("\r{}", line);
                    std::io::stdout().flush().unwrap_or(());
                }
            }
        }

        let status = step2_child.wait()?;
        if !status.success() || had_error {
            return Err("Step 2: FFmpeg audio combination failed".into());
        }
    } else {
        let status = step2_child.wait()?;
        if !status.success() {
            return Err("Step 2: FFmpeg audio combination failed".into());
        }
    }

    // --- NEW: emit thumbnail next to the .mp4 ---
    let _thumb_path = write_thumbnail(
        &image_input_path,
        &config.audio_path,
        &config.output_path,
        config.verbose,
    )?;

    // Clean up temporary file(s)
    if Path::new(temp_video_path).exists() {
        let _ = std::fs::remove_file(temp_video_path);
    }
    if let Some(p) = temp_cover_to_delete {
        let _ = std::fs::remove_file(p);
    }

    // Verify the output file
    if let Ok(metadata) = std::fs::metadata(&config.output_path) {
        if metadata.len() > 0 {
            println!(
                "\nVideo created successfully! Output: {} ({} bytes)",
                config.output_path,
                metadata.len()
            );
            Ok(())
        } else {
            Err("Output file was created but has zero size".into())
        }
    } else {
        Err("Failed to create output file".into())
    }
}

// -------------------------------
// Batch runner
// -------------------------------

fn derive_output_path(
    audio_path: &str,
    out_dir: &Option<String>,
) -> Result<String, Box<dyn Error>> {
    let mut out = PathBuf::from(audio_path);
    out.set_extension("mp4");
    if let Some(dir) = out_dir {
        let file = out.file_name().ok_or("Invalid audio file name")?.to_owned();
        let mut dst = PathBuf::from(dir);
        if !dst.exists() {
            std::fs::create_dir_all(&dst)?;
        }
        dst.push(file);
        Ok(dst.to_string_lossy().into_owned())
    } else {
        Ok(out.to_string_lossy().into_owned())
    }
}

fn run_batch(app: AppConfig) -> Result<(), Box<dyn Error>> {
    for audio in app.inputs {
        let output = derive_output_path(&audio, &app.out_dir)?;
        println!("Processing: {}", audio);

        let cfg = VideoConfig {
            image_path: app.shared.image_path.clone(),
            audio_path: audio.clone(),
            output_path: output,
            viz_type: app.shared.viz_type,
            duration: app.shared.duration,
            position: app.shared.position,
            color_scheme: app.shared.color_scheme,
            width: app.shared.width,
            height: app.shared.height,
            margin: app.shared.margin,
            verbose: app.shared.verbose,
            cover_from_audio: app.shared.cover_from_audio,
            cover_out: app.shared.cover_out.clone(), // ignored if batch
        };

        create_video(cfg)?;
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    // Check if ffmpeg is available
    if Command::new("ffmpeg").arg("-version").output().is_err() {
        return Err(
            "FFmpeg not found. Please install FFmpeg and make sure it's in your PATH.".into(),
        );
    }

    match parse_args()? {
        Some(app) => run_batch(app)?,
        None => return Ok(()),
    }

    Ok(())
}

// -------------------------------
// Tests
// -------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use id3::TagLike;
    use serial_test::serial;
    use std::fs;
    use std::path::Path;
    use std::process::Stdio;
    use std::thread;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    #[derive(Debug)]
    struct VideoValidation {
        has_video: bool,
        has_audio: bool,
        duration: f64,
        video_codec: String,
        audio_codec: String,
    }

    #[derive(Debug)]
    enum TestError {
        Io(std::io::Error),
        Ffmpeg(String),
        Validation(String),
    }

    impl std::error::Error for TestError {}
    impl std::fmt::Display for TestError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                TestError::Io(e) => write!(f, "IO error: {}", e),
                TestError::Ffmpeg(s) => write!(f, "FFmpeg error: {}", s),
                TestError::Validation(s) => write!(f, "Validation error: {}", s),
            }
        }
    }

    struct TestFiles {
        image_path: String,
        audio_path: String,
        output_path: String,
        cleaned_up: bool,
    }

    impl TestFiles {
        fn new() -> Result<Self, TestError> {
            // Unique test dir
            let test_dir = format!(
                "test_files_{}_{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            );

            fs::create_dir_all(&test_dir).map_err(TestError::Io)?;

            let files = TestFiles {
                image_path: format!("{}/test_image.png", test_dir),
                audio_path: format!("{}/test_audio.mp3", test_dir),
                output_path: format!("{}/test_output.mp4", test_dir),
                cleaned_up: false,
            };

            files.generate_test_files()?;
            files.verify_files()?;
            Ok(files)
        }

        fn cleanup(&mut self) {
            if !self.cleaned_up {
                if let Some(test_dir) = Path::new(&self.image_path).parent() {
                    if test_dir.exists() {
                        let _ = fs::remove_dir_all(test_dir);
                    }
                }
                self.cleaned_up = true;
            }
        }

        fn generate_test_files(&self) -> Result<(), TestError> {
            // Image
            let status = Command::new("ffmpeg")
                .arg("-y")
                .arg("-f")
                .arg("lavfi")
                .arg("-i")
                .arg("color=c=black:s=1280x720")
                .arg("-frames:v")
                .arg("1")
                .arg(&self.image_path)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map_err(TestError::Io)?;
            if !status.success() {
                return Err(TestError::Ffmpeg("Failed to generate test image".into()));
            }

            // Audio (no cover—tests that pass an explicit image will use it)
            let status = Command::new("ffmpeg")
                .arg("-y")
                .arg("-f")
                .arg("lavfi")
                .arg("-i")
                .arg("sine=frequency=440:duration=3")
                .arg("-c:a")
                .arg("libmp3lame")
                .arg(&self.audio_path)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map_err(TestError::Io)?;
            if !status.success() {
                return Err(TestError::Ffmpeg("Failed to generate test audio".into()));
            }

            Ok(())
        }

        fn verify_files(&self) -> Result<(), TestError> {
            for (file_type, path) in [("Image", &self.image_path), ("Audio", &self.audio_path)] {
                if !Path::new(path).exists() {
                    return Err(TestError::Validation(format!(
                        "{} file not found at {}",
                        file_type, path
                    )));
                }
                let metadata = fs::metadata(path).map_err(TestError::Io)?;
                println!("{} file size: {} bytes", file_type, metadata.len());
            }
            Ok(())
        }
    }

    impl Drop for TestFiles {
        fn drop(&mut self) {
            self.cleanup();
        }
    }

    fn validate_video_file(path: &str) -> Result<VideoValidation, TestError> {
        // Delay to ensure file closed
        thread::sleep(Duration::from_secs(1));

        // Verify file exists/size
        let metadata = fs::metadata(path)
            .map_err(|e| TestError::Validation(format!("Failed to get file metadata: {}", e)))?;
        if metadata.len() == 0 {
            return Err(TestError::Validation("Output file has zero size".into()));
        }

        let output = Command::new("ffprobe")
            .arg("-v")
            .arg("error")
            .arg("-show_streams")
            .arg("-show_format")
            .arg("-of")
            .arg("json")
            .arg(path)
            .output()
            .map_err(TestError::Io)?;

        if !output.status.success() {
            println!(
                "FFprobe stderr: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            return Err(TestError::Ffmpeg("FFprobe command failed".into()));
        }

        let probe_output: serde_json::Value = serde_json::from_slice(&output.stdout)
            .map_err(|e| TestError::Validation(format!("JSON parse error: {}", e)))?;

        let mut validation = VideoValidation {
            has_video: false,
            has_audio: false,
            duration: 0.0,
            video_codec: String::new(),
            audio_codec: String::new(),
        };

        if let Some(streams) = probe_output.get("streams").and_then(|s| s.as_array()) {
            for stream in streams {
                if let Some(codec_type) = stream.get("codec_type").and_then(|t| t.as_str()) {
                    match codec_type {
                        "video" => {
                            validation.has_video = true;
                            if let Some(codec_name) =
                                stream.get("codec_name").and_then(|n| n.as_str())
                            {
                                validation.video_codec = codec_name.to_string();
                            }
                        }
                        "audio" => {
                            validation.has_audio = true;
                            if let Some(codec_name) =
                                stream.get("codec_name").and_then(|n| n.as_str())
                            {
                                validation.audio_codec = codec_name.to_string();
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        if let Some(format) = probe_output.get("format") {
            if let Some(duration) = format.get("duration").and_then(|d| d.as_str()) {
                validation.duration = duration.parse().unwrap_or(0.0);
            }
        }

        Ok(validation)
    }

    // ---------------- Tests ----------------

    #[test]
    #[serial]
    fn test_spectrum_visualization() -> Result<(), Box<dyn Error>> {
        let mut files = TestFiles::new()?;

        if let Some(parent) = Path::new(&files.output_path).parent() {
            fs::create_dir_all(parent)?;
        }

        let config = VideoConfig {
            image_path: Some(files.image_path.clone()),
            audio_path: files.audio_path.clone(),
            output_path: files.output_path.clone(),
            viz_type: VisualizationType::Spectrum,
            duration: Some(2.0),
            position: VisualizationPosition::Bottom,
            color_scheme: SpectrumColorScheme::Fire,
            width: 1280,
            height: 180,
            margin: 50,
            verbose: true,
            cover_from_audio: false,
            cover_out: None,
        };

        create_video(config)?;

        thread::sleep(Duration::from_secs(1));

        let validation = validate_video_file(&files.output_path)?;
        assert!(validation.has_video, "Video stream not found");
        assert!(validation.has_audio, "Audio stream not found");
        assert!(
            validation.duration > 0.0,
            "Duration should be greater than 0"
        );
        assert!(
            !validation.video_codec.is_empty(),
            "Video codec should not be empty"
        );
        assert!(
            !validation.audio_codec.is_empty(),
            "Audio codec should not be empty"
        );

        files.cleanup();
        Ok(())
    }

    #[test]
    #[serial]
    fn test_both_visualizations() -> Result<(), Box<dyn Error>> {
        let mut files = TestFiles::new()?;

        if let Some(parent) = Path::new(&files.output_path).parent() {
            fs::create_dir_all(parent)?;
        }

        let config = VideoConfig {
            image_path: Some(files.image_path.clone()),
            audio_path: files.audio_path.clone(),
            output_path: files.output_path.clone(),
            viz_type: VisualizationType::Both,
            duration: Some(2.0),
            position: VisualizationPosition::Bottom,
            color_scheme: SpectrumColorScheme::Cool,
            width: 1280,
            height: 360,
            margin: 50,
            verbose: true,
            cover_from_audio: false,
            cover_out: None,
        };

        create_video(config)?;

        thread::sleep(Duration::from_secs(1));

        let validation = validate_video_file(&files.output_path)?;
        assert!(validation.has_video, "Video stream not found");
        assert!(validation.has_audio, "Audio stream not found");
        assert!(
            validation.duration > 0.0,
            "Duration should be greater than 0"
        );
        assert!(
            !validation.video_codec.is_empty(),
            "Video codec should not be empty"
        );
        assert!(
            !validation.audio_codec.is_empty(),
            "Audio codec should not be empty"
        );

        files.cleanup();
        Ok(())
    }

    #[test]
    #[serial]
    fn test_waveform_visualization() -> Result<(), Box<dyn Error>> {
        let mut files = TestFiles::new()?;

        if let Some(parent) = Path::new(&files.output_path).parent() {
            fs::create_dir_all(parent)?;
        }

        let config = VideoConfig {
            image_path: Some(files.image_path.clone()),
            audio_path: files.audio_path.clone(),
            output_path: files.output_path.clone(),
            viz_type: VisualizationType::Waveform,
            duration: Some(2.0),
            position: VisualizationPosition::Bottom,
            color_scheme: SpectrumColorScheme::Rainbow,
            width: 1280,
            height: 180,
            margin: 50,
            verbose: true,
            cover_from_audio: false,
            cover_out: None,
        };

        create_video(config)?;

        thread::sleep(Duration::from_secs(1));

        let validation = validate_video_file(&files.output_path)?;
        assert!(validation.has_video, "Video stream not found");
        assert!(validation.has_audio, "Audio stream not found");
        assert!(
            validation.duration > 0.0,
            "Duration should be greater than 0"
        );
        assert!(
            !validation.video_codec.is_empty(),
            "Video codec should not be empty"
        );
        assert!(
            !validation.audio_codec.is_empty(),
            "Audio codec should not be empty"
        );

        files.cleanup();
        Ok(())
    }

    #[test]
    fn test_visualization_type_parsing() {
        assert!(matches!(
            "wave".parse::<VisualizationType>(),
            Ok(VisualizationType::Waveform)
        ));
        assert!(matches!(
            "waveform".parse::<VisualizationType>(),
            Ok(VisualizationType::Waveform)
        ));
        assert!(matches!(
            "spectrum".parse::<VisualizationType>(),
            Ok(VisualizationType::Spectrum)
        ));
        assert!(matches!(
            "spec".parse::<VisualizationType>(),
            Ok(VisualizationType::Spectrum)
        ));
        assert!(matches!(
            "both".parse::<VisualizationType>(),
            Ok(VisualizationType::Both)
        ));
        assert!("invalid".parse::<VisualizationType>().is_err());
    }

    #[test]
    fn test_spectrum_color_scheme_parsing() {
        assert!(matches!(
            "rainbow".parse::<SpectrumColorScheme>(),
            Ok(SpectrumColorScheme::Rainbow)
        ));
        assert!(matches!(
            "moreland".parse::<SpectrumColorScheme>(),
            Ok(SpectrumColorScheme::Moreland)
        ));
        assert!(matches!(
            "nebulae".parse::<SpectrumColorScheme>(),
            Ok(SpectrumColorScheme::Nebulae)
        ));
        assert!(matches!(
            "fire".parse::<SpectrumColorScheme>(),
            Ok(SpectrumColorScheme::Fire)
        ));
        assert!(matches!(
            "fiery".parse::<SpectrumColorScheme>(),
            Ok(SpectrumColorScheme::Fiery)
        ));
        assert!(matches!(
            "fruit".parse::<SpectrumColorScheme>(),
            Ok(SpectrumColorScheme::Fruit)
        ));
        assert!(matches!(
            "cool".parse::<SpectrumColorScheme>(),
            Ok(SpectrumColorScheme::Cool)
        ));
        assert!(matches!(
            "magma".parse::<SpectrumColorScheme>(),
            Ok(SpectrumColorScheme::Magma)
        ));
        assert!(matches!(
            "green".parse::<SpectrumColorScheme>(),
            Ok(SpectrumColorScheme::Green)
        ));
        assert!(matches!(
            "viridis".parse::<SpectrumColorScheme>(),
            Ok(SpectrumColorScheme::Viridis)
        ));
        assert!(matches!(
            "plasma".parse::<SpectrumColorScheme>(),
            Ok(SpectrumColorScheme::Plasma)
        ));
        assert!(matches!(
            "cividis".parse::<SpectrumColorScheme>(),
            Ok(SpectrumColorScheme::Cividis)
        ));
        assert!(matches!(
            "terrain".parse::<SpectrumColorScheme>(),
            Ok(SpectrumColorScheme::Terrain)
        ));
        assert!("invalid".parse::<SpectrumColorScheme>().is_err());
    }

    #[test]
    fn test_visualization_position_parsing() {
        assert!(matches!(
            "top".parse::<VisualizationPosition>(),
            Ok(VisualizationPosition::Top)
        ));
        assert!(matches!(
            "bottom".parse::<VisualizationPosition>(),
            Ok(VisualizationPosition::Bottom)
        ));
        assert!(matches!(
            "left".parse::<VisualizationPosition>(),
            Ok(VisualizationPosition::Left)
        ));
        assert!(matches!(
            "right".parse::<VisualizationPosition>(),
            Ok(VisualizationPosition::Right)
        ));
        assert!(matches!(
            "center".parse::<VisualizationPosition>(),
            Ok(VisualizationPosition::Center)
        ));

        if let Ok(VisualizationPosition::Custom(x, y)) =
            "xy(10,20)".parse::<VisualizationPosition>()
        {
            assert_eq!(x, 10);
            assert_eq!(y, 20);
        } else {
            panic!("Failed to parse custom position");
        }

        // Test invalid custom formats
        assert!("xy(10)".parse::<VisualizationPosition>().is_err());
        assert!("xy(a,b)".parse::<VisualizationPosition>().is_err());
        assert!("xy(10,20".parse::<VisualizationPosition>().is_err());
        assert!("invalid".parse::<VisualizationPosition>().is_err());
    }

    #[test]
    fn test_get_spectrum_params() {
        let (w, h, orient) = get_spectrum_params(VisualizationPosition::Left, 100, 200);
        assert_eq!(w, 200);
        assert_eq!(h, 100);
        assert_eq!(orient, "vertical");

        let (w, h, orient) = get_spectrum_params(VisualizationPosition::Right, 100, 200);
        assert_eq!(w, 200);
        assert_eq!(h, 100);
        assert_eq!(orient, "vertical");

        let (w, h, orient) = get_spectrum_params(VisualizationPosition::Top, 100, 200);
        assert_eq!(w, 100);
        assert_eq!(h, 200);
        assert_eq!(orient, "horizontal");
    }

    #[test]
    fn test_get_position_overlay() {
        assert_eq!(
            get_position_overlay(VisualizationPosition::Top, 10),
            "x=(W-w)/2:y=10"
        );
        assert_eq!(
            get_position_overlay(VisualizationPosition::Bottom, 10),
            "x=(W-w)/2:y=H-h-10"
        );
        assert_eq!(
            get_position_overlay(VisualizationPosition::Left, 10),
            "x=10:y=(H-h)/2"
        );
        assert_eq!(
            get_position_overlay(VisualizationPosition::Right, 10),
            "x=W-w-10:y=(H-h)/2"
        );
        assert_eq!(
            get_position_overlay(VisualizationPosition::Center, 10),
            "x=(W-w)/2:y=(H-h)/2"
        );
        assert_eq!(
            get_position_overlay(VisualizationPosition::Custom(5, 5), 10),
            "x=5:y=5"
        );
    }

    #[test]
    fn test_ext_from_mime() {
        assert_eq!(ext_from_mime("image/jpeg"), "jpg");
        assert_eq!(ext_from_mime("image/jpg"), "jpg");
        assert_eq!(ext_from_mime("image/png"), "png");
        assert_eq!(ext_from_mime("image/webp"), "webp");
        assert_eq!(ext_from_mime("unknown/type"), "bin");
    }

    #[test]
    fn test_derive_output_path() {
        // Simple case
        let input = if cfg!(windows) {
            "music\\song.mp3"
        } else {
            "music/song.mp3"
        };
        let expected = if cfg!(windows) {
            "music\\song.mp4"
        } else {
            "music/song.mp4"
        };
        assert_eq!(
            PathBuf::from(derive_output_path(input, &None).unwrap()),
            PathBuf::from(expected)
        );

        // With output dir
        let temp_dir = std::env::temp_dir().join(format!("test_derive_{}", std::process::id()));
        let temp_dir_str = temp_dir.to_string_lossy().into_owned();

        let res = derive_output_path("song.mp3", &Some(temp_dir_str.clone())).unwrap();
        assert_eq!(PathBuf::from(res), temp_dir.join("song.mp4"));
        assert!(temp_dir.exists());

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_parse_args_basic() -> Result<(), Box<dyn Error>> {
        let dummy_file = format!("test_arg_parse_{}.mp3", std::process::id());
        std::fs::write(&dummy_file, "dummy")?;

        let args = vec![
            "prog".to_string(),
            dummy_file.clone(),
            "--type".to_string(),
            "spectrum".to_string(),
        ];
        let config = parse_args_from(args)?.unwrap();

        assert_eq!(config.inputs.len(), 1);
        assert!(matches!(
            config.shared.viz_type,
            VisualizationType::Spectrum
        ));

        std::fs::remove_file(dummy_file)?;
        Ok(())
    }

    #[test]
    fn test_parse_args_glob() -> Result<(), Box<dyn Error>> {
        let dummy1 = format!("test_glob_1_{}.mp3", std::process::id());
        let dummy2 = format!("test_glob_2_{}.mp3", std::process::id());
        std::fs::write(&dummy1, "d")?;
        std::fs::write(&dummy2, "d")?;

        let args = vec![
            "prog".to_string(),
            format!("test_glob_*_{}.mp3", std::process::id()),
        ];
        let config = parse_args_from(args)?.unwrap();

        // Glob order is not guaranteed, but we should find 2
        assert_eq!(config.inputs.len(), 2);

        std::fs::remove_file(dummy1)?;
        std::fs::remove_file(dummy2)?;
        Ok(())
    }

    #[test]
    fn test_parse_args_missing_file() {
        let args = vec![
            "prog".to_string(),
            "non_existent_file_99999.mp3".to_string(),
        ];
        assert!(parse_args_from(args).is_err());
    }

    #[test]
    fn test_parse_args_invalid_option() {
        let dummy_file = format!("test_arg_invalid_{}.mp3", std::process::id());
        std::fs::write(&dummy_file, "dummy").unwrap();

        let args = vec![
            "prog".to_string(),
            dummy_file.clone(),
            "--invalid-opt".to_string(),
        ];
        assert!(parse_args_from(args).is_err());

        std::fs::remove_file(dummy_file).unwrap();
    }

    #[test]
    fn test_parse_args_batch_warning() -> Result<(), Box<dyn Error>> {
        let dummy1 = format!("test_batch_1_{}.mp3", std::process::id());
        let dummy2 = format!("test_batch_2_{}.mp3", std::process::id());
        std::fs::write(&dummy1, "d")?;
        std::fs::write(&dummy2, "d")?;

        let args = vec![
            "prog".to_string(),
            format!("test_batch_*_{}.mp3", std::process::id()),
            "--cover-out".to_string(),
            "cover.jpg".to_string(),
        ];
        let config = parse_args_from(args)?.unwrap();

        // Glob order is not guaranteed, but we should find 2
        assert_eq!(config.inputs.len(), 2);
        assert!(config.shared.cover_out.is_none()); // Should be cleared

        std::fs::remove_file(dummy1)?;
        std::fs::remove_file(dummy2)?;
        Ok(())
    }

    #[test]
    #[serial]
    fn test_cover_extraction_workflow() -> Result<(), Box<dyn Error>> {
        let mut files = TestFiles::new()?;

        // Generate a valid JPEG for cover
        let cover_gen_path = files.image_path.replace(".png", ".jpg");
        let status = std::process::Command::new("ffmpeg")
            .arg("-y")
            .arg("-f")
            .arg("lavfi")
            .arg("-i")
            .arg("color=c=blue:s=640x640")
            .arg("-frames:v")
            .arg("1")
            .arg(&cover_gen_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        assert!(status.success(), "Failed to generate cover jpg");

        let jpeg_data = std::fs::read(&cover_gen_path)?;
        // We can delete the source jpg now
        std::fs::remove_file(&cover_gen_path)?;

        // Rewrite audio file with ID3 tag containing a picture
        let mut tag = id3::Tag::new();
        tag.add_frame(id3::frame::Picture {
            mime_type: "image/jpeg".to_string(),
            picture_type: id3::frame::PictureType::CoverFront,
            description: "Cover".to_string(),
            data: jpeg_data,
        });
        // We need to write to the existing MP3 file
        tag.write_to_path(&files.audio_path, id3::Version::Id3v23)?;

        let config = VideoConfig {
            image_path: None,
            audio_path: files.audio_path.clone(),
            output_path: files.output_path.clone(),
            viz_type: VisualizationType::Waveform,
            duration: Some(1.0),
            position: VisualizationPosition::Bottom,
            color_scheme: SpectrumColorScheme::Viridis,
            width: 1280,
            height: 720,
            margin: 50,
            verbose: true,
            cover_from_audio: true,
            cover_out: Some(files.output_path.replace(".mp4", ".jpg")),
        };

        create_video(config)?;

        // Verify cover output
        let cover_path = files.output_path.replace(".mp4", ".jpg");
        assert!(
            Path::new(&cover_path).exists(),
            "Cover image was not extracted"
        );

        // Verify video
        let validation = validate_video_file(&files.output_path)?;
        assert!(validation.has_video);

        // Cleanup extra
        std::fs::remove_file(cover_path).unwrap_or(());
        files.cleanup();
        Ok(())
    }

    #[test]
    fn test_print_usage() {
        print_usage();
    }

    #[test]
    #[serial]
    fn test_run_batch_execution() -> Result<(), Box<dyn Error>> {
        // Create 2 dummy audio files
        let mut files = TestFiles::new()?;
        let dir = Path::new(&files.audio_path).parent().unwrap();
        let audio2 = dir.join("test_audio_2.mp3");

        // Copy audio1 to audio2
        std::fs::copy(&files.audio_path, &audio2)?;

        let app = AppConfig {
            inputs: vec![
                files.audio_path.clone(),
                audio2.to_string_lossy().into_owned(),
            ],
            out_dir: None,
            shared: SharedOpts {
                image_path: Some(files.image_path.clone()),
                viz_type: VisualizationType::Waveform,
                duration: Some(0.1), // Very short for speed
                position: VisualizationPosition::Bottom,
                color_scheme: SpectrumColorScheme::Viridis,
                width: 100,
                height: 100,
                margin: 10,
                verbose: false,
                cover_from_audio: false,
                cover_out: None,
            },
        };

        run_batch(app)?;

        // Verify both outputs exist
        // run_batch derives output name from input name.
        // input: test_audio.mp3 -> output: test_audio.mp4
        // TestFiles.output_path is "test_output.mp4", which is NOT what run_batch produces.
        let expected_output1 = Path::new(&files.audio_path).with_extension("mp4");
        assert!(expected_output1.exists(), "Expected batch output 1 missing");

        let output2 = audio2.with_extension("mp4");
        assert!(output2.exists(), "Expected batch output 2 missing");

        // Cleanup
        let _ = std::fs::remove_file(expected_output1);
        let _ = std::fs::remove_file(output2);
        let _ = std::fs::remove_file(audio2);
        files.cleanup();
        Ok(())
    }

    #[test]
    #[serial]
    fn test_cover_extraction_ffmpeg_fallback() -> Result<(), Box<dyn Error>> {
        let mut files = TestFiles::new()?;

        // 1. Create a simple MP4 video file (which id3 crate cannot read).
        // This will test the ffmpeg fallback logic in extract_cover_to_file.
        let video_path = files.audio_path.replace(".mp3", ".mp4");
        let status = std::process::Command::new("ffmpeg")
            .arg("-y")
            .arg("-f")
            .arg("lavfi")
            .arg("-i")
            .arg("color=c=blue:s=320x240:d=1")
            .arg("-f")
            .arg("lavfi")
            .arg("-i")
            .arg("sine=f=440:d=1")
            .arg("-c:v")
            .arg("libx264")
            .arg("-c:a")
            .arg("aac")
            .arg("-t")
            .arg("1")
            .arg(&video_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        assert!(status.success());

        // Verify id3 crate fails on this file
        let tag_check = id3::Tag::read_from_path(&video_path);
        assert!(tag_check.is_err(), "ID3 crate should fail on MP4 container");

        let config = VideoConfig {
            image_path: None,
            audio_path: video_path.clone(),
            output_path: files.output_path.clone(),
            viz_type: VisualizationType::Waveform,
            duration: Some(0.1),
            position: VisualizationPosition::Center,
            color_scheme: SpectrumColorScheme::Viridis,
            width: 100,
            height: 100,
            margin: 0,
            verbose: true,
            cover_from_audio: true, // Force extraction from the "audio" (video) file
            cover_out: None,
        };

        create_video(config)?;

        // Verify video created
        assert!(Path::new(&files.output_path).exists());

        // Cleanup
        let _ = std::fs::remove_file(video_path);
        files.cleanup();
        Ok(())
    }

    #[test]
    #[serial]
    fn test_error_handling() -> Result<(), Box<dyn Error>> {
        let mut files = TestFiles::new()?;

        // Case 1: Missing Audio
        let config_missing = VideoConfig {
            image_path: Some(files.image_path.clone()),
            audio_path: "non_existent_audio.mp3".to_string(),
            output_path: "should_fail.mp4".to_string(),
            ..VideoConfig::default()
        };
        assert!(create_video(config_missing).is_err());

        // Case 2: FFmpeg failure (invalid image path provided explicitly)
        let config_bad_img = VideoConfig {
            image_path: Some("non_existent_image.png".to_string()),
            audio_path: files.audio_path.clone(),
            output_path: "should_fail_2.mp4".to_string(),
            ..VideoConfig::default()
        };
        // This fails because we check if image exists inside create_video logic
        assert!(create_video(config_bad_img).is_err());

        // Case 3: FFmpeg execution failure (corrupt audio)
        let corrupt_audio = files.audio_path.replace(".mp3", "_corrupt.mp3");
        std::fs::write(&corrupt_audio, "not an mp3 file")?;

        let config_corrupt = VideoConfig {
            image_path: Some(files.image_path.clone()),
            audio_path: corrupt_audio.clone(),
            output_path: "should_fail_3.mp4".to_string(),
            ..VideoConfig::default()
        };
        assert!(create_video(config_corrupt).is_err());

        let _ = std::fs::remove_file(corrupt_audio);
        files.cleanup();
        Ok(())
    }
}
