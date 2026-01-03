# mdmp3tomp4

`mdmp3tomp4` is a high-performance, Rust-based command-line tool that converts audio files into visually engaging video files. It leverages the power of **FFmpeg** to generate real-time audio visualizations (Waveforms, Spectrums) and combines them with background images or embedded album art.

Designed for content creators, musicians, and archivists, it supports single-file processing and high-speed batch conversion.

## Features

*   **Visualizations**: Choose between **Waveform**, **Spectrum**, or **Both**.
*   **Batch Processing**: Convert entire directories or match files using glob patterns (e.g., `*.mp3`).
*   **Cover Art Extraction**: Automatically extracts embedded cover art from audio files (ID3 tags, FLAC metadata, MP4 atoms) to use as the video background.
*   **Customization**:
    *   **Color Schemes**: 13+ presets including Viridis, Magma, Rainbow, Fire, and more.
    *   **Positioning**: Place visualizations at the Top, Bottom, Left, Right, Center, or exact XY coordinates.
    *   **Dimensions**: Control width, height, and margins.
*   **Thumbnails**: Automatically generates a high-quality video thumbnail alongside the output.
*   **Robust**: Handles missing metadata and falls back gracefully.

## Prerequisites

1.  **Rust Toolchain**: Required to build the project. [Install Rust](https://www.rust-lang.org/tools/install).
2.  **FFmpeg**: This tool wraps FFmpeg for media processing. It must be installed and available in your system's `PATH`.
    *   **Windows**: `winget install ffmpeg` or download from [gyan.dev](https://www.gyan.dev/ffmpeg/builds/).
    *   **macOS**: `brew install ffmpeg`
    *   **Linux**: `sudo apt install ffmpeg`

## Installation

Clone the repository and build using Cargo:

```bash
git clone https://github.com/your-username/mdmp3tomp4.git
cd mdmp3tomp4
cargo build --release
```

The binary will be located at `target/release/mdmp3tomp4`.

## Usage

Basic syntax:
```bash
mdmp3tomp4 <input_file_or_glob> [options]
```

### Examples

**1. Simple Conversion**
Convert a single song using default settings (Waveform, dark background if no cover found):
```bash
mdmp3tomp4 song.mp3
```

**2. Batch Processing**
Convert all MP3 files in the current directory:
```bash
mdmp3tomp4 "*.mp3"
```

**3. Use Embedded Cover Art**
Extract the album art from the audio file and use it as the background:
```bash
mdmp3tomp4 song.mp3 --cover-from-audio
```

**4. Custom Visualization**
Create a **Spectrum** visualization with the **Fire** color scheme, positioned at the **Bottom**:
```bash
mdmp3tomp4 song.mp3 --type spectrum --color fire --position bottom
```

**5. Advanced Layout**
Render **Both** waveform and spectrum, use a custom background image, and save to a specific directory:
```bash
mdmp3tomp4 music/*.flac --image background.jpg --out-dir ./rendered_videos --type both --color viridis
```

## Options Reference

| Flag | Description | Default |
| :--- | :--- | :--- |
| `input` | The audio file path or glob pattern (e.g., `*.mp3`). | (Required) |
| `--out-dir <dir>` | Directory to write output files. | Same as input |
| `--image <path>` | Path to a background image. | Black background |
| `--cover-from-audio`| Attempt to extract embedded cover art to use as background. | `false` |
| `--cover-out <path>`| Save the extracted cover art to a file (Single mode only). | `None` |
| `--type <type>` | Visualization type: `wave`, `spectrum`, `both`. | `wave` |
| `--color <scheme>` | Color scheme (see below). | `viridis` |
| `--position <pos>` | Position: `top`, `bottom`, `left`, `right`, `center`, `xy(x,y)`. | `bottom` |
| `--width <px>` | Width of the visualization. | `1280` |
| `--height <px>` | Height of the visualization. | `180` |
| `--margin <px>` | Margin from the edge. | `50` |
| `--duration <sec>` | Limit video duration (useful for previews). | Full Length |
| `--verbose` | Print detailed FFmpeg output. | `false` |

### Color Schemes
Available palettes for the spectrum visualization:
*   `rainbow`, `moreland`, `nebulae`, `fire`, `fiery`, `fruit`, `cool`, `magma`, `green`, `viridis`, `plasma`, `cividis`, `terrain`.

## Development

### Running Tests
The project includes a comprehensive test suite covering CLI parsing, fallback logic, and integration with FFmpeg.

```bash
cargo test
```

### Architecture
*   **`src/main.rs`**: Monolithic entry point containing argument parsing, configuration logic, and the FFmpeg command builder.
*   **Integration Tests**: Located in `src/main.rs` (under `mod tests`), these tests create temporary audio/image assets to verify the full rendering pipeline without external dependencies (other than FFmpeg).

## License

[MIT License](LICENSE)
