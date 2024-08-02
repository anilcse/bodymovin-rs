use bodymovin::{get_all_frames, save_frame};
use std::{fs::create_dir_all, time::Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let start = Instant::now();

  let bodymovin_json = "assets/bodymovin.json";
  let assets_dir = "assets/images";
  let output_dir = "output_frames";

  // Get frames
  let frames = get_all_frames(bodymovin_json, assets_dir)?;

  println!("Got the frames in time: {:?}", start.elapsed());

  let start = Instant::now();

  // Create output directory if it doesn't exist
  create_dir_all(output_dir)?;

  // Save each frame
  for (frame_number, frame) in frames.into_iter().enumerate() {
    save_frame(&frame, output_dir, frame_number as u32)?;
  }

  println!("Total rendering and saving time: {:?}", start.elapsed());

  Ok(())
}
