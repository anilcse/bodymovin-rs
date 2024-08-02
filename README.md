
# Bodymovin Renderer

**Bodymovin Renderer** is a Rust library for rendering frames from Bodymovin JSON data and handling video frames. This library provides utilities for processing animations, converting image buffers to video frames, and more.

## Features

- Parse Bodymovin JSON data to extract animation details.
- Render frames from animations using image buffers.
- Convert image buffers into video frames with timestamp handling.
- Easily customizable frame rate and time base for accurate video rendering.

## Installation

To use this library, add the following to your `Cargo.toml`:

```toml
[dependencies]
image = "0.24" # Or the latest version
num-rational = "0.4" # Or the latest version
rayon = "1.7" # Or the latest version
serde = { version = "1.0", features = ["derive"] }
```

## Usage

Below is a basic example demonstrating how to render frames from Bodymovin JSON and convert them into video frames:

```rust
use bodymovin_renderer::bodymovin::{get_all_frames, save_frame, BodymovinError};

fn main() -> Result<(), BodymovinError> {
    // Paths to the necessary files and directories
    let bodymovin_json = "path/to/bodymovin.json";
    let assets_dir = "path/to/bodymovin/assets";
    let output_dir = "path/to/output";

    // Render all frames from Bodymovin JSON
    let frames = get_all_frames(&bodymovin_json, &assets_dir)?;

    
    // Save each frame
    for (frame_number, frame) in frames.into_iter().enumerate() {
        save_frame(&frame, output_dir, frame_number as u32)?;
    }

    Ok(())
}
```

### Example Code

This example code demonstrates how to use the library to process frames and convert them into a video-friendly format:

1. **Rendering Frames**: Using `get_all_frames` to render frames from a Bodymovin JSON file.
2. **Saving Frames**: Using the `save_frame` function to store processed frames in a specified directory.

### Important Types and Functions

- **`get_all_frames`**: Function to render frames from Bodymovin JSON.

## Contributing

We welcome contributions! Feel free to submit issues or pull requests to improve the library.

### To Do

- Implement additional features for more animation effects.
- Improve error handling and performance optimizations.
- Enhance documentation with more examples and use cases.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
