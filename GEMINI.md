# mdmp3tomp4

## Project Overview
`mdmp3tomp4` is a Rust-based command-line tool that converts audio files (MP3) into video files (MP4) featuring audio visualizations. It leverages `ffmpeg` to generate the video content, supporting various visualization types (Waveform, Spectrum, Both) and customizations.

### Key Features
*   **Visualizations:** Waveform, Spectrum, or both.
*   **Customization:** Configurable colors, positions, and dimensions.
*   **Cover Art:** Can extract cover art from MP3 tags or use an external image.
*   **Batch Processing:** Supports glob patterns for batch conversion.

## Architecture
The project is a single-crate Rust application.
*   **`src/main.rs`**: Contains the entire application logic, including CLI argument parsing, configuration, `ffmpeg` command construction, and testing.
*   **Dependencies**:
    *   `ffmpeg` (External): The core video processing engine. Must be installed and available in the system PATH.
    *   `id3`: For reading MP3 metadata and cover art.
    *   `glob`: For file pattern matching.
    *   `serde`/`serde_json`: Used in tests for validating `ffprobe` output.

## Building and Running

### Prerequisites
*   **Rust:** Stable toolchain.
*   **FFmpeg:** `ffmpeg` and `ffprobe` must be installed and in your system's PATH.

### Build Commands
```bash
# Build for release
cargo build --release
```

### Usage
```bash
# Basic usage
cargo run -- song.mp3

# Batch processing
cargo run -- "*.mp3"

# Custom visualization
cargo run -- song.mp3 --type spectrum --color fire --position bottom
```

See `cargo run -- --help` (or the `print_usage` function in `src/main.rs`) for a full list of options.

### Testing
The project includes a comprehensive test suite in `src/main.rs`:
*   **Integration Tests:** Verify the full video creation pipeline (generating assets, running ffmpeg, checking output) for all visualization types and cover extraction.
*   **Unit Tests:** Cover CLI argument parsing, enum parsing, path derivation, and helper functions.

Tests run sequentially using `serial_test` to prevent I/O conflicts with temporary files.

```bash
cargo test
```

## Development Conventions
*   **Code Structure:** Currently monolithic `main.rs`. Future refactoring might split concerns (CLI, config, video generation).
*   **Testing:** Tests use `serial_test` to ensure they run sequentially, preventing conflicts with temporary file usage. Tests generate their own input assets (black video, sine wave audio) using `ffmpeg` filters.
*   **FFmpeg Integration:** The tool constructs and executes `ffmpeg` commands via `std::process::Command`. Output is parsed for validation.
