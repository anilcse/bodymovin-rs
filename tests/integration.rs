use bodymovin::{get_all_frames, BodymovinError};
use std::fs::{create_dir_all, remove_dir_all, File};
use std::io::Write;
use std::path::Path;

// Helper function to create a temporary directory and files
fn setup_test_environment() -> Result<(String, String, String), Box<dyn std::error::Error>> {
    let assets_dir = Path::new("assets");
    let bodymovin_json_path = assets_dir.join("bodymovin.json");
    let assets_dir_path = assets_dir.join("images");
    let output_dir_path = assets_dir.join("output_frames");

    // Create necessary directories
    create_dir_all(&assets_dir_path)?;
    create_dir_all(&output_dir_path)?;

    Ok((
        bodymovin_json_path.to_str().unwrap().to_string(),
        assets_dir_path.to_str().unwrap().to_string(),
        output_dir_path.to_str().unwrap().to_string(),
    ))
}

#[test]
fn test_get_all_frames() -> Result<(), BodymovinError> {
    // Setup test environment
    let (bodymovin_json, assets_dir, _output_dir) = setup_test_environment().expect("Failed to set up test environment");

    // Test get_all_frames
    let frames = get_all_frames(&bodymovin_json, &assets_dir)?;
    
    // Check that frames are returned (we'll just check that we have the expected number)
    assert_eq!(frames.len(), 774);

    Ok(())
}