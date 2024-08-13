use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba};
use imageproc::geometric_transformations::rotate_about_center;
use rayon::prelude::*;
use serde_json::Value;
use std::{
  collections::HashMap,
  error::Error,
  fmt,
  fs::File,
  io::BufReader,
  ops::{Add, Mul},
  path::Path,
  sync::Arc,
};

// Define a custom error type
#[derive(Debug)]
pub enum BodymovinError {
  IoError(std::io::Error),
  ImageError(image::ImageError),
  JsonError(serde_json::Error),
  OtherError(String),
}

#[derive(Clone, Copy, Debug)]
struct Vec2 {
  x: f32,
  y: f32,
}

impl Vec2 {
  fn new(x: f32, y: f32) -> Self {
    Vec2 { x, y }
  }
}

impl Add for Vec2 {
  type Output = Self;
  fn add(self, other: Self) -> Self {
    Vec2::new(self.x + other.x, self.y + other.y)
  }
}

impl Mul<f32> for Vec2 {
  type Output = Self;
  fn mul(self, scalar: f32) -> Self {
    Vec2::new(self.x * scalar, self.y * scalar)
  }
}

struct Layer {
  start_frame: f32,
  end_frame: f32,
  transform: Value,
  asset_id: Option<String>,
}

struct Asset {
  image: Arc<DynamicImage>,
}

#[derive(Clone, Debug)]
struct Transform {
    position: Vec2,
    anchor_point: Vec2,
    scale: Vec2,
    rotation: f32,
    opacity: f32,
}

impl fmt::Display for BodymovinError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      BodymovinError::IoError(err) => write!(f, "IO error: {}", err),
      BodymovinError::ImageError(err) => write!(f, "Image error: {}", err),
      BodymovinError::JsonError(err) => write!(f, "JSON error: {}", err),
      BodymovinError::OtherError(err) => write!(f, "Error: {}", err),
    }
  }
}

impl Error for BodymovinError {}

impl From<std::io::Error> for BodymovinError {
  fn from(err: std::io::Error) -> Self {
    BodymovinError::IoError(err)
  }
}

impl From<image::ImageError> for BodymovinError {
  fn from(err: image::ImageError) -> Self {
    BodymovinError::ImageError(err)
  }
}

impl From<serde_json::Error> for BodymovinError {
  fn from(err: serde_json::Error) -> Self {
    BodymovinError::JsonError(err)
  }
}

// Update function signatures to use BodymovinError
fn load_bodymovin_json(path: &str) -> Result<Value, BodymovinError> {
  let file = File::open(path)?;
  let reader = BufReader::new(file);
  Ok(serde_json::from_reader(reader)?)
}

fn load_assets(
  assets_path: &str,
  animation_data: &Value,
) -> Result<HashMap<String, Asset>, BodymovinError> {
  let mut assets = HashMap::new();
  if let Some(assets_array) = animation_data["assets"].as_array() {
    for asset in assets_array {
      let id = asset["id"]
        .as_str()
        .ok_or_else(|| BodymovinError::OtherError("Missing asset id".to_string()))?;
      let path = format!(
        "{}/{}",
        assets_path,
        asset["p"]
          .as_str()
          .ok_or_else(|| BodymovinError::OtherError("Missing asset path".to_string()))?
      );
      let image = Arc::new(image::open(&path)?);
      assets.insert(id.to_string(), Asset { image });
    }
  }
  Ok(assets)
}


fn render_frame(
  width: u32,
  height: u32,
  assets: &HashMap<String, Asset>,
  layers: &[Layer],
  frame_number: u32,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, BodymovinError> {
  let mut image = ImageBuffer::new(width, height);

  for layer in layers.iter().rev() {
      if frame_number as f32 >= layer.start_frame && frame_number as f32 <= layer.end_frame {
          if let Some(asset_id) = &layer.asset_id {
              if let Some(asset) = assets.get(asset_id) {
                  let transform = parse_transform(&layer.transform, frame_number as f32)?;
                  composite_layer(&mut image, &asset.image, &transform);
              }
          }
      }
  }

  Ok(image)
}

fn composite_layer(
    base_image: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    layer_image: &DynamicImage,
    transform: &Transform,
) {
    let (width, height) = layer_image.dimensions();

    println!("scale:{:?}", transform.scale);
    let mut scale = transform.scale;

    if scale.x.abs() < 1.0 {
      scale.x = 1.0;
      scale.y = 1.0;
    }

    let scaled_width = (width as f32 * scale.x.abs()) as u32;
    let scaled_height = (height as f32 * scale.y.abs()) as u32;

    let mut resized_image = layer_image.resize(
        scaled_width,
        scaled_height,
        image::imageops::FilterType::Lanczos3,
    );

    if transform.rotation != 0.0 {
        resized_image = DynamicImage::ImageRgba8(rotate_about_center(
            &resized_image.to_rgba8(),
            transform.rotation.to_radians(),
            imageproc::geometric_transformations::Interpolation::Bilinear,
            Rgba([0, 0, 0, 0]),
        ));
    }

    // Calculate the top-left corner position
    let anchor_x = transform.anchor_point.x ;
    let anchor_y = transform.anchor_point.y ;
    let x = transform.position.x - anchor_x;
    let y = transform.position.y - anchor_y;
    

    image::imageops::overlay(base_image, &resized_image, x as i64, y as i64);

    if transform.opacity < 1.0 {
        for pixel in base_image.pixels_mut() {
            let alpha = (pixel[3] as f32 * transform.opacity) as u8;
            *pixel = Rgba([pixel[0], pixel[1], pixel[2], alpha]);
        }
    }
}

fn parse_vec2(v: &Value) -> Vec2 {
  if v.is_array() {
      Vec2 {
          x: v[0].as_f64().unwrap_or(0.0) as f32,
          y: v[1].as_f64().unwrap_or(0.0) as f32,
      }
  } else if v.is_object() {
      Vec2 {
          x: v["x"].as_f64().unwrap_or(0.0) as f32,
          y: v["y"].as_f64().unwrap_or(0.0) as f32,
      }
  } else {
      Vec2 { x: 0.0, y: 0.0 }
  }
}

fn parse_transform(transform_data: &Value, frame: f32) -> Result<Transform, BodymovinError> {
  let position = if let Some(p) = transform_data.get("p") {
      if p["a"].as_i64().unwrap_or(0) == 1 {
          // Animated position
          interpolate_vec2(&p["k"], frame)
      } else {
          // Static position
          parse_vec2(&p["k"])
      }
  } else {
      Vec2 { x: 0.0, y: 0.0 }
  };

  let scale = if let Some(s) = transform_data.get("s") {
      if s["a"].as_i64().unwrap_or(0) == 1 {
          // Animated scale
          let scale_vec = interpolate_vec2(&s["k"], frame);
          Vec2 {
              x: scale_vec.x / 100.0,
              y: scale_vec.y / 100.0,
          }
      } else {
          // Static scale
          let scale_vec = parse_vec2(&s["k"]);
          Vec2 {
              x: scale_vec.x / 100.0,
              y: scale_vec.y / 100.0,
          }
      }
  } else {
      Vec2 { x: 1.0, y: 1.0 }
  };

  let anchor_point = if let Some(a) = transform_data.get("a") {
    if a["a"].as_i64().unwrap_or(0) == 1 {
        // Animated anchor point
        interpolate_vec2(&a["k"], frame)
    } else {
        // Static anchor point
        parse_vec2(&a["k"])
    }
} else {
    Vec2 { x: 0.0, y: 0.0 }
};

  let rotation = if let Some(r) = transform_data.get("r") {
      if r["a"].as_i64().unwrap_or(0) == 1 {
          // Animated rotation
          interpolate_f32(&r["k"], frame)
      } else {
          // Static rotation
          r["k"].as_f64().unwrap_or(0.0) as f32
      }
  } else {
      0.0
  };

  let opacity = if let Some(o) = transform_data.get("o") {
      if o["a"].as_i64().unwrap_or(0) == 1 {
          // Animated opacity
          (interpolate_f32(&o["k"], frame) / 100.0).clamp(0.0, 1.0)
      } else {
          // Static opacity
          (o["k"].as_f64().unwrap_or(100.0) as f32 / 100.0).clamp(0.0, 1.0)
      }
  } else {
      1.0
  };

  Ok(Transform {
      position,
      anchor_point,
      scale,
      rotation,
      opacity,
  })
}

fn interpolate_vec2(keyframes: &Value, frame: f32) -> Vec2 {
  if let Some(keyframes_array) = keyframes.as_array() {
      // Find the keyframes before and after the current frame
      let mut prev_keyframe = &keyframes_array[0];
      let mut next_keyframe = &keyframes_array[0];

      for keyframe in keyframes_array {
          if keyframe["t"].as_f64().unwrap_or(0.0) as f32 <= frame {
              prev_keyframe = keyframe;
          } else {
              next_keyframe = keyframe;
              break;
          }
      }

      let start_time = prev_keyframe["t"].as_f64().unwrap_or(0.0) as f32;
      let end_time = next_keyframe["t"].as_f64().unwrap_or(0.0) as f32;
      let progress = (frame - start_time) / (end_time - start_time);

      let start_value = parse_vec2(&prev_keyframe["s"]);
      let end_value = parse_vec2(&next_keyframe["e"]);

      Vec2 {
          x: start_value.x + (end_value.x - start_value.x) * progress,
          y: start_value.y + (end_value.y - start_value.y) * progress,
      }
  } else {
      parse_vec2(keyframes)
  }
}

fn interpolate_f32(keyframes: &Value, frame: f32) -> f32 {
  if let Some(keyframes_array) = keyframes.as_array() {
      // Find the keyframes before and after the current frame
      let mut prev_keyframe = &keyframes_array[0];
      let mut next_keyframe = &keyframes_array[0];

      for keyframe in keyframes_array {
          if keyframe["t"].as_f64().unwrap_or(0.0) as f32 <= frame {
              prev_keyframe = keyframe;
          } else {
              next_keyframe = keyframe;
              break;
          }
      }

      let start_time = prev_keyframe["t"].as_f64().unwrap_or(0.0) as f32;
      let end_time = next_keyframe["t"].as_f64().unwrap_or(0.0) as f32;
      let progress = (frame - start_time) / (end_time - start_time);

      let start_value = prev_keyframe["s"][0].as_f64().unwrap_or(0.0) as f32;
      let end_value = next_keyframe["e"][0].as_f64().unwrap_or(0.0) as f32;

      start_value + (end_value - start_value) * progress
  } else {
      keyframes.as_f64().unwrap_or(0.0) as f32
  }
}

fn parse_layers(animation_data: &Value) -> Result<Vec<Layer>, BodymovinError> {
  animation_data["layers"]
      .as_array()
      .ok_or_else(|| BodymovinError::OtherError("No layers found".to_string()))?
      .iter()
      .map(|layer| {
          Ok(Layer {
              start_frame: layer["ip"].as_f64().unwrap_or(0.0) as f32,
              end_frame: layer["op"].as_f64().unwrap_or(0.0) as f32,
              transform: layer["ks"].clone(),
              asset_id: layer["refId"].as_str().map(String::from),
          })
      })
      .collect()
}

pub fn save_frame(
  frame: &ImageBuffer<Rgba<u8>, Vec<u8>>,
  output_dir: &str,
  frame_number: u32,
) -> Result<(), BodymovinError> {
  let file_name = format!("frame_{:04}.png", frame_number);
  let path = Path::new(output_dir).join(file_name);
  frame.save(path).map_err(BodymovinError::ImageError)?;
  Ok(())
}

pub fn get_all_frames(bodymovin_json: &str, assets_dir: &str) -> Result<Vec<ImageBuffer<Rgba<u8>, Vec<u8>>>, BodymovinError> {
  let animation_data = load_bodymovin_json(bodymovin_json)?;
  let assets = load_assets(assets_dir, &animation_data)?;
  let layers = parse_layers(&animation_data)?;

  let width = animation_data["w"].as_u64().unwrap_or(540) as u32;
  let height = animation_data["h"].as_u64().unwrap_or(800) as u32;
  let total_frames = animation_data["op"].as_f64().unwrap_or(0.0) as u32;

  // Render frames in parallel and collect them into a vector
  let frames: Result<Vec<_>, _> = (0..total_frames)
      .into_par_iter()
      .map(|frame_number| render_frame(width, height, &assets, &layers, frame_number))
      .collect();

  frames
}