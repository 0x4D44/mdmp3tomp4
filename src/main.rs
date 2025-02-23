use std::process::{Command, Stdio};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::env;
use std::io::{BufRead, Write, BufReader};

#[derive(Debug, Clone, Copy)]
enum VisualizationType {
    Waveform,
    Spectrum,
    Both
}

impl std::str::FromStr for VisualizationType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "wave" | "waveform" => Ok(VisualizationType::Waveform),
            "spectrum" | "spec" => Ok(VisualizationType::Spectrum),
            "both" => Ok(VisualizationType::Both),
            _ => Err(format!("Unknown visualization type: {}. Use 'wave', 'spectrum', or 'both'.", s))
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
    Terrain
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
    Custom(u32, u32)
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
            _ if s.starts_with("xy(") => {
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

#[derive(Debug, Clone)]
struct VideoConfig {
    image_path: String,
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
}

impl Default for VideoConfig {
    fn default() -> Self {
        Self {
            image_path: String::new(),
            audio_path: String::new(),
            output_path: String::new(),
            viz_type: VisualizationType::Both,
            duration: None,
            position: VisualizationPosition::Bottom,
            color_scheme: SpectrumColorScheme::Viridis,
            width: 1280,
            height: 180,
            margin: 50,
            verbose: false,
        }
    }
}

fn print_usage() {
    println!("Usage: mp3tomp4 <image_path> <audio_path> <output_path> [options]");
    println!("\nOptions:");
    println!("  --type <type>       Visualization type: 'wave', 'spectrum', or 'both' (default: both)");
    println!("  --duration <sec>    Maximum duration in seconds (optional)");
    println!("  --position <pos>    Position: 'top', 'bottom', 'left', 'right', 'center', or 'xy(x,y)' (default: bottom)");
    println!("  --color <scheme>    Color scheme: 'rainbow', 'moreland', 'nebulae', 'fire', 'fiery', 'fruit', 'cool',");
    println!("                      'magma', 'green', 'viridis', 'plasma', 'cividis', 'terrain' (default: viridis)");

    println!("  --width <pixels>    Visualization width in pixels (default: 1280)");
    println!("  --height <pixels>   Visualization height in pixels (default: 180)");
    println!("  --margin <pixels>   Margin from edges in pixels (default: 50)");
    println!("  --verbose           Enable detailed FFmpeg output");
    println!("\nExamples:");
    println!("  mp3tomp4 input.jpg music.mp3 output.mp4 --type both --position bottom --color fire");
    println!("  mp3tomp4 image.png audio.wav video.mp4 --type spectrum --position right --color rgb(255,0,0) --width 360 --height 720");
    println!(" ");
}

fn parse_args() -> Result<Option<VideoConfig>, Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 4 {
        print_usage();
        return Ok(None);
    }

    let mut config = VideoConfig {
        image_path: args[1].clone(),
        audio_path: args[2].clone(),
        output_path: args[3].clone(),
        ..Default::default()
    };

    let mut i = 4;
    while i < args.len() {
        match args[i].as_str() {
            "--type" => {
                i += 1;
                if i < args.len() {
                    config.viz_type = args[i].parse()?;
                }
            },
            "--duration" => {
                i += 1;
                if i < args.len() {
                    config.duration = Some(args[i].parse()?);
                }
            },
            "--position" => {
                i += 1;
                if i < args.len() {
                    config.position = args[i].parse()?;
                }
            },
            "--color" => {
                i += 1;
                if i < args.len() {
                    config.color_scheme = args[i].parse()?;
                }
            },
            "--width" => {
                i += 1;
                if i < args.len() {
                    config.width = args[i].parse()?;
                }
            },
            "--height" => {
                i += 1;
                if i < args.len() {
                    config.height = args[i].parse()?;
                }
            },
            "--margin" => {
                i += 1;
                if i < args.len() {
                    config.margin = args[i].parse()?;
                }
            },
            "--verbose" => {
                config.verbose = true;
            },
            unknown => return Err(format!("Unknown argument: {}", unknown).into()),
        }
        i += 1;
    }

    Ok(Some(config))
}

// Helper function to get spectrum parameters based on orientation
fn get_spectrum_params(pos: VisualizationPosition, width: u32, height: u32) -> (u32, u32, &'static str) {
    match pos {
        VisualizationPosition::Left | VisualizationPosition::Right => 
            (height, width, "vertical"),
        _ => (width, height, "horizontal")
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
                base, config.width, config.height,
                get_position_overlay(config.position, config.margin)
            )
        },
        VisualizationType::Spectrum => {
            let (spec_width, spec_height, orientation) = get_spectrum_params(config.position, config.width, config.height);
            let spec_params = get_color_args(config.color_scheme, spec_width, spec_height, orientation);
            
            format!(
                "{}; \
                [1:a]aformat=channel_layouts=mono,showspectrum={}[spec]; \
                [bg][spec]overlay={}",
                base, spec_params,
                get_position_overlay(config.position, config.margin)
            )
        },
        VisualizationType::Both => {
            let gap = config.margin / 2; // Dynamic gap based on margin
            let (wave_height, spec_height) = match config.position {
                VisualizationPosition::Left | VisualizationPosition::Right => {
                    (config.width / 2, config.width / 2) // For vertical layout
                },
                _ => (config.height / 2, config.height / 2) // For horizontal layout
            };
            
            let (spec_width, spec_height, orientation) = get_spectrum_params(config.position, config.width, spec_height);
            let spec_params = get_color_args(config.color_scheme, spec_width, spec_height, orientation);
            
            let (wave_pos, spec_pos) = match config.position {
                VisualizationPosition::Bottom => (
                    format!("x=(W-w)/2:y=H-h-{}-{}", spec_height + gap, config.margin),
                    format!("x=(W-w)/2:y=H-h-{}", config.margin)
                ),
                VisualizationPosition::Top => (
                    format!("x=(W-w)/2:y={}", config.margin),
                    format!("x=(W-w)/2:y={}+{}", wave_height + gap + config.margin, config.margin)
                ),
                VisualizationPosition::Left => (
                    format!("x={}:y=(H-h)/2", config.margin),
                    format!("x={}+{}:y=(H-h)/2", wave_height + gap + config.margin, config.margin)
                ),
                VisualizationPosition::Right => (
                    format!("x=W-w-{}-{}:y=(H-h)/2", spec_width + gap, config.margin),
                    format!("x=W-w-{}:y=(H-h)/2", config.margin)
                ),
                VisualizationPosition::Center => (
                    format!("x=(W-w)/2:y=(H-h)/2-{}", wave_height/2 + gap/2),
                    format!("x=(W-w)/2:y=(H-h)/2+{}", gap/2)
                ),
                VisualizationPosition::Custom(x, y) => (
                    format!("x={}:y={}", x, y),
                    format!("x={}:y={}+{}", x, y + wave_height, gap)
                )
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
fn get_color_args(scheme: SpectrumColorScheme, width: u32, height: u32, orientation: &str) -> String {
    let base_args = format!(
        "s={}x{}:mode=combined:scale=cbrt:slide=scroll:fscale=lin:\
        win_func=hamming:overlap=0:fps=auto:start=100:stop=10000:orientation={}",
        width, height,
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
        SpectrumColorScheme::Terrain => "terrain"
    };
    
    format!("{}:color={}", base_args, color)
}

fn get_position_overlay(pos: VisualizationPosition, margin: u32) -> String {
    match pos {
        VisualizationPosition::Top => 
            format!("x=(W-w)/2:y={}", margin),
        VisualizationPosition::Bottom => 
            format!("x=(W-w)/2:y=H-h-{}", margin),
        VisualizationPosition::Left => 
            format!("x={}:y=(H-h)/2", margin),
        VisualizationPosition::Right => 
            format!("x=W-w-{}:y=(H-h)/2", margin),
        VisualizationPosition::Center => 
            "x=(W-w)/2:y=(H-h)/2".to_string(),
        VisualizationPosition::Custom(x, y) => 
            format!("x={}:y={}", x, y)
    }
}

fn create_video(config: VideoConfig) -> Result<(), Box<dyn Error>> {
    // Validate input files
    if !Path::new(&config.image_path).exists() {
        return Err(format!("Image file not found: {}", config.image_path).into());
    }
    if !Path::new(&config.audio_path).exists() {
        return Err(format!("Audio file not found: {}", config.audio_path).into());
    }

    // Get audio duration
    let duration = Command::new("ffprobe")
        .arg("-v").arg("error")
        .arg("-show_entries").arg("format=duration")
        .arg("-of").arg("default=noprint_wrappers=1:nokey=1")
        .arg(&config.audio_path)
        .output()?;

    let audio_duration: f32 = String::from_utf8_lossy(&duration.stdout)
        .trim()
        .parse()
        .unwrap_or(0.0);

    let target_duration = config.duration.unwrap_or(audio_duration);

    // Create temporary file with a unique name
    let temp_video = PathBuf::from(env::temp_dir()).join(format!("temp_video_{}.mp4", std::process::id()));
    let temp_video_path = temp_video.to_str().ok_or("Failed to create temporary path")?;

    println!("Creating temporary file at: {}", temp_video_path);

    // Generate the filter complex string using our new function
    let filter = get_filter_complex(&config);

    println!("Step 1: Creating visualization video...");

    let mut step1 = Command::new("ffmpeg");
    step1.arg("-y")
         .arg("-i").arg(&config.image_path)
         .arg("-i").arg(&config.audio_path)
         .arg("-filter_complex").arg(&filter)
         .arg("-c:v").arg("libx264")
         .arg("-c:a").arg("aac")
         .arg("-preset").arg("ultrafast")
         .arg("-tune").arg("stillimage")
         .arg("-t").arg(target_duration.to_string())
         .arg("-pix_fmt").arg("yuv420p")
         .arg(temp_video_path);

    if !config.verbose {
        step1.stderr(Stdio::piped());
    }

    let mut step1_child = step1.spawn()?;
    
    if !config.verbose {
        let mut had_error = false;
        if let Some(stderr) = step1_child.stderr.take() {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                if let Ok(line) = line {
                    if line.contains("Error") || line.contains("error") {
                        println!("FFmpeg error: {}", line);
                        had_error = true;
                    } else if line.contains("frame=") || line.contains("time=") {
                        print!("\r{}", line);
                        std::io::stdout().flush().unwrap_or(());
                    }
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
    step2.arg("-y")
         .arg("-i").arg(temp_video_path)
         .arg("-i").arg(&config.audio_path)
         .arg("-map").arg("0:v:0")
         .arg("-map").arg("1:a:0")
         .arg("-c:v").arg("copy")
         .arg("-c:a").arg("aac")
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
            for line in reader.lines() {
                if let Ok(line) = line {
                    if line.contains("Error") || line.contains("error") {
                        println!("FFmpeg error: {}", line);
                        had_error = true;
                    } else if line.contains("frame=") || line.contains("time=") {
                        print!("\r{}", line);
                        std::io::stdout().flush().unwrap_or(());
                    }
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

    // Clean up temporary file
    if Path::new(temp_video_path).exists() {
        let _ = std::fs::remove_file(temp_video_path);
    }

    // Verify the output file
    if let Ok(metadata) = std::fs::metadata(&config.output_path) {
        if metadata.len() > 0 {
            println!("\nVideo created successfully! Output size: {} bytes", metadata.len());
            Ok(())
        } else {
            Err("Output file was created but has zero size".into())
        }
    } else {
        Err("Failed to create output file".into())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // Check if ffmpeg is available
    if let Err(_) = Command::new("ffmpeg").arg("-version").output() {
        return Err("FFmpeg not found. Please install FFmpeg and make sure it's in your PATH.".into());
    }

    match parse_args()? {
        Some(config) => create_video(config)?,
        None => return Ok(()),
    }

    Ok(())
}                    

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Stdio;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};
    use std::thread;
    use std::time::Duration;
    use serde_json;
    use serial_test::serial;

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
        cleaned_up: bool,  // Add this field to track cleanup state
    }

    impl TestFiles {
        fn new() -> Result<Self, TestError> {
            // Generate a unique test directory name using process ID and timestamp
            let test_dir = format!("test_files_{}_{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            );
            
            println!("Creating test directory: {}", test_dir);
            
            // Create test directory and all parent directories
            fs::create_dir_all(&test_dir)
                .map_err(|e| TestError::Io(e))?;
            
            let files = TestFiles {
                image_path: format!("{}/test_image.png", test_dir),
                audio_path: format!("{}/test_audio.mp3", test_dir),
                output_path: format!("{}/test_output.mp4", test_dir),
                cleaned_up: false,  // Initialize as not cleaned up
            };
            
            println!("Generating test files...");
            files.generate_test_files()?;
            files.verify_files()?;
            println!("Test files generated and verified successfully");
            
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
            // Generate test image
            let status = Command::new("ffmpeg")
                .arg("-y")
                .arg("-f").arg("lavfi")
                .arg("-i").arg("color=c=black:s=1280x720")
                .arg("-frames:v").arg("1")
                .arg(&self.image_path)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map_err(|e| TestError::Io(e))?;

            if !status.success() {
                return Err(TestError::Ffmpeg("Failed to generate test image".into()));
            }

            // Generate test audio
            let status = Command::new("ffmpeg")
                .arg("-y")
                .arg("-f").arg("lavfi")
                .arg("-i").arg("sine=frequency=440:duration=3")
                .arg("-c:a").arg("libmp3lame")
                .arg(&self.audio_path)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map_err(|e| TestError::Io(e))?;

            if !status.success() {
                return Err(TestError::Ffmpeg("Failed to generate test audio".into()));
            }

            Ok(())
        }

        fn verify_files(&self) -> Result<(), TestError> {
            for (file_type, path) in [
                ("Image", &self.image_path),
                ("Audio", &self.audio_path),
            ] {
                if !Path::new(path).exists() {
                    return Err(TestError::Validation(
                        format!("{} file not found at {}", file_type, path)
                    ));
                }
                let metadata = fs::metadata(path)
                    .map_err(|e| TestError::Io(e))?;
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
        // Add delay to ensure file is fully written
        thread::sleep(Duration::from_secs(1));
        
        // Verify file exists and has size
        let metadata = fs::metadata(path)
            .map_err(|e| TestError::Validation(format!("Failed to get file metadata: {}", e)))?;
            
        if metadata.len() == 0 {
            return Err(TestError::Validation("Output file has zero size".into()));
        }

        println!("Probing file: {} (size: {} bytes)", path, metadata.len());

        let output = Command::new("ffprobe")
            .arg("-v").arg("error")
            .arg("-show_streams")
            .arg("-show_format")
            .arg("-of").arg("json")
            .arg(path)
            .output()
            .map_err(|e| TestError::Io(e))?;
    
        if !output.status.success() {
            println!("FFprobe stderr: {}", String::from_utf8_lossy(&output.stderr));
            return Err(TestError::Ffmpeg("FFprobe command failed".into()));
        }
    
        println!("FFprobe output: {}", String::from_utf8_lossy(&output.stdout));
    
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
                            if let Some(codec_name) = stream.get("codec_name").and_then(|n| n.as_str()) {
                                validation.video_codec = codec_name.to_string();
                            }
                        },
                        "audio" => {
                            validation.has_audio = true;
                            if let Some(codec_name) = stream.get("codec_name").and_then(|n| n.as_str()) {
                                validation.audio_codec = codec_name.to_string();
                            }
                        },
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
    
        println!("Validation results: {:?}", validation);
        Ok(validation)
    }

    #[test]
    fn test_command_line_parsing() -> Result<(), Box<dyn Error>> {
        // Test minimal args
        let args = vec![
            "program".to_string(),
            "input.jpg".to_string(),
            "audio.mp3".to_string(),
            "output.mp4".to_string(),
        ];

        let config = parse_args_with_args(&args)?;
        assert!(config.is_some());
        if let Some(config) = config {
            assert_eq!(config.image_path, "input.jpg");
            assert_eq!(config.audio_path, "audio.mp3");
            assert_eq!(config.output_path, "output.mp4");
        }

        // Test all options
        let args = vec![
            "program".to_string(),
            "input.jpg".to_string(),
            "audio.mp3".to_string(),
            "output.mp4".to_string(),
            "--type".to_string(), "spectrum".to_string(),
            "--position".to_string(), "right".to_string(),
            "--color".to_string(), "fire".to_string(),
            "--width".to_string(), "360".to_string(),
            "--height".to_string(), "720".to_string(),
            "--margin".to_string(), "30".to_string(),
            "--verbose".to_string(),
        ];

        let config = parse_args_with_args(&args)?;
        assert!(config.is_some());
        if let Some(config) = config {
            assert_eq!(config.image_path, "input.jpg");
            assert_eq!(config.audio_path, "audio.mp3");
            assert_eq!(config.output_path, "output.mp4");
            matches!(config.viz_type, VisualizationType::Spectrum);
            matches!(config.position, VisualizationPosition::Right);
            matches!(config.color_scheme, SpectrumColorScheme::Fire);
            assert_eq!(config.width, 360);
            assert_eq!(config.height, 720);
            assert_eq!(config.margin, 30);
            assert!(config.verbose);
        }

        Ok(())
    }

    #[test]
    fn test_invalid_arguments() -> Result<(), Box<dyn Error>> {
        // Test invalid visualization type
        let args = vec![
            "program".to_string(),
            "input.jpg".to_string(),
            "audio.mp3".to_string(),
            "output.mp4".to_string(),
            "--type".to_string(),
            "invalid".to_string(),
        ];
        assert!(parse_args_with_args(&args).is_err());

        // Test invalid position
        let args = vec![
            "program".to_string(),
            "input.jpg".to_string(),
            "audio.mp3".to_string(),
            "output.mp4".to_string(),
            "--position".to_string(),
            "invalid".to_string(),
        ];
        assert!(parse_args_with_args(&args).is_err());

        // Test invalid color scheme
        let args = vec![
            "program".to_string(),
            "input.jpg".to_string(),
            "audio.mp3".to_string(),
            "output.mp4".to_string(),
            "--color".to_string(),
            "invalid".to_string(),
        ];
        assert!(parse_args_with_args(&args).is_err());

        Ok(())
    }

    #[test]
    #[serial]
    fn test_spectrum_visualization() -> Result<(), Box<dyn Error>> {
        let mut files = TestFiles::new()?;
        println!("Test files created successfully");
        
        // Create parent directory for output file if it doesn't exist
        if let Some(parent) = Path::new(&files.output_path).parent() {
            fs::create_dir_all(parent)?;
        }
        
        let config = VideoConfig {
            image_path: files.image_path.clone(),
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
        };

        println!("Starting create_video with config: {:?}", config);
        create_video(config)?;
        println!("Video creation completed");
        
        thread::sleep(Duration::from_secs(1));
        
        // Prevent cleanup until after validation
        let validation = validate_video_file(&files.output_path)?;
        println!("Video validation completed: {:?}", validation);
        
        assert!(validation.has_video, "Video stream not found");
        assert!(validation.has_audio, "Audio stream not found");
        assert!(validation.duration > 0.0, "Duration should be greater than 0");
        assert!(!validation.video_codec.is_empty(), "Video codec should not be empty");
        assert!(!validation.audio_codec.is_empty(), "Audio codec should not be empty");

        // Now we can cleanup
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
            image_path: files.image_path.clone(),
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
        };

        println!("Starting create_video with config: {:?}", config);
        create_video(config)?;
        println!("Video creation completed");
        
        thread::sleep(Duration::from_secs(1));
        
        // Prevent cleanup until after validation
        let validation = validate_video_file(&files.output_path)?;
        println!("Video validation completed: {:?}", validation);
        
        assert!(validation.has_video, "Video stream not found");
        assert!(validation.has_audio, "Audio stream not found");
        assert!(validation.duration > 0.0, "Duration should be greater than 0");
        assert!(!validation.video_codec.is_empty(), "Video codec should not be empty");
        assert!(!validation.audio_codec.is_empty(), "Audio codec should not be empty");

        // Now we can cleanup
        files.cleanup();
        
        Ok(())
    }

    #[test]
    #[serial]
    fn test_waveform_visualization() -> Result<(), Box<dyn Error>> {
        let mut files = TestFiles::new()?;
        println!("Test files created successfully");
        
        if let Some(parent) = Path::new(&files.output_path).parent() {
            fs::create_dir_all(parent)?;
        }
        
        let config = VideoConfig {
            image_path: files.image_path.clone(),
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
        };

        println!("Starting create_video with config: {:?}", config);
        create_video(config)?;
        println!("Video creation completed");
        
        thread::sleep(Duration::from_secs(1));
        
        // Prevent cleanup until after validation
        let validation = validate_video_file(&files.output_path)?;
        println!("Video validation completed: {:?}", validation);
        
        assert!(validation.has_video, "Video stream not found");
        assert!(validation.has_audio, "Audio stream not found");
        assert!(validation.duration > 0.0, "Duration should be greater than 0");
        assert!(!validation.video_codec.is_empty(), "Video codec should not be empty");
        assert!(!validation.audio_codec.is_empty(), "Audio codec should not be empty");

        // Now we can cleanup
        files.cleanup();
        
        Ok(())
    }
    
    // Helper function for command-line parsing tests
    fn parse_args_with_args(args: &[String]) -> Result<Option<VideoConfig>, Box<dyn Error>> {
        if args.len() < 4 {
            return Ok(None);
        }

        let mut config = VideoConfig {
            image_path: args[1].clone(),
            audio_path: args[2].clone(),
            output_path: args[3].clone(),
            ..Default::default()
        };

        let mut i = 4;
        while i < args.len() {
            match args[i].as_str() {
                "--type" => {
                    i += 1;
                    if i < args.len() {
                        config.viz_type = args[i].parse()?;
                    }
                },
                "--position" => {
                    i += 1;
                    if i < args.len() {
                        config.position = args[i].parse()?;
                    }
                },
                "--color" => {
                    i += 1;
                    if i < args.len() {
                        config.color_scheme = args[i].parse()?;
                    }
                },
                "--width" => {
                    i += 1;
                    if i < args.len() {
                        config.width = args[i].parse()?;
                    }
                },
                "--height" => {
                    i += 1;
                    if i < args.len() {
                        config.height = args[i].parse()?;
                    }
                },
                "--margin" => {
                    i += 1;
                    if i < args.len() {
                        config.margin = args[i].parse()?;
                    }
                },
                "--verbose" => {
                    config.verbose = true;
                },
                "--duration" => {
                    i += 1;
                    if i < args.len() {
                        config.duration = Some(args[i].parse()?);
                    }
                },
                unknown => return Err(format!("Unknown argument: {}", unknown).into()),
            }
            i += 1;
        }

        Ok(Some(config))
    }
}                    