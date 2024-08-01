use serde_json::Value;
use std::fs::File;
use std::io::BufReader;
use image::{ImageBuffer, Rgba, DynamicImage};
use std::collections::HashMap;
use image::GenericImageView;
use std::ops::{Add, Mul};

#[derive(Clone, Debug)]
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
        Vec2 {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;

    fn mul(self, scalar: f32) -> Self {
        Vec2 {
            x: self.x * scalar,
            y: self.y * scalar,
        }
    }
}

struct Layer {
    start_frame: f32,
    end_frame: f32,
    in_point: f32,
    out_point: f32,
    transform: Transform,
    asset_id: Option<String>,
}

struct Transform {
    position: Keyframes<Vec2>,
    scale: Keyframes<Vec2>,
    rotation: Keyframes<f32>,
    opacity: Keyframes<f32>,
}

enum Keyframes<T> {
    Static(T),
    Animated(Vec<Keyframe<T>>),
}

struct Keyframe<T> {
    time: f32,
    value: T,
}

struct Asset {
    id: String,
    path: String,
    width: u32,
    height: u32,
}

fn load_bodymovin_json(path: &str) -> Result<Value, Box<dyn std::error::Error>> {
  let file = File::open(path)?;
  let reader = BufReader::new(file);
  let json: Value = serde_json::from_reader(reader)?;
  Ok(json)
}

fn render_frame(animation_data: &Value, assets_path: &str, frame_number: u32) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, Box<dyn std::error::Error>> {
  let width = animation_data["w"].as_u64().unwrap_or(540) as u32;
  let height = animation_data["h"].as_u64().unwrap_or(800) as u32;
  let mut image = ImageBuffer::new(width, height);

  let assets = load_assets(assets_path, animation_data)?;
  let layers = parse_layers(animation_data)?;

  for layer in layers.iter().rev() {
      if frame_number as f32 >= layer.start_frame && frame_number as f32 <= layer.end_frame {
          if let Some(asset_id) = &layer.asset_id {
              if let Some(asset) = assets.get(asset_id) {
                  let asset_image = image::open(&asset.path)?;
                  composite_layer(&mut image, &asset_image, layer, frame_number as f32)?;
              }
          }
      }
  }

  Ok(image)
}

fn load_assets(assets_path: &str, animation_data: &Value) -> Result<HashMap<String, Asset>, Box<dyn std::error::Error>> {
    let mut assets = HashMap::new();
    if let Some(assets_array) = animation_data["assets"].as_array() {
        for asset in assets_array {
            let id = asset["id"].as_str().unwrap_or("").to_string();
            let path = format!("{}/{}", assets_path, asset["p"].as_str().unwrap_or(""));
            let width = asset["w"].as_u64().unwrap_or(0) as u32;
            let height = asset["h"].as_u64().unwrap_or(0) as u32;
            println!("path:{}",path);
            assets.insert(id.clone(), Asset { id, path, width, height });
        }
    }
    Ok(assets)
}

fn parse_layers(animation_data: &Value) -> Result<Vec<Layer>, Box<dyn std::error::Error>> {
    let mut layers = Vec::new();
    if let Some(layers_array) = animation_data["layers"].as_array() {
        for layer in layers_array {
            let start_frame = layer["ip"].as_f64().unwrap_or(0.0) as f32;
            let end_frame = layer["op"].as_f64().unwrap_or(0.0) as f32;
            let in_point = layer["ip"].as_f64().unwrap_or(0.0) as f32;
            let out_point = layer["op"].as_f64().unwrap_or(0.0) as f32;
            let asset_id = layer["refId"].as_str().map(String::from);
            
            let transform = parse_transform(&layer["ks"])?;

            layers.push(Layer {
                start_frame,
                end_frame,
                in_point,
                out_point,
                transform,
                asset_id,
            });
        }
    }
    Ok(layers)
}

fn parse_transform(transform_data: &Value) -> Result<Transform, Box<dyn std::error::Error>> {
  Ok(Transform {
      position: parse_keyframes(&transform_data["p"], |v| Vec2::new(v[0].as_f64().unwrap_or(0.0) as f32, v[1].as_f64().unwrap_or(0.0) as f32))?,
      scale: parse_keyframes(&transform_data["s"], |v| Vec2::new(v[0].as_f64().unwrap_or(100.0) as f32, v[1].as_f64().unwrap_or(100.0) as f32))?,
      rotation: parse_keyframes(&transform_data["r"], |v| v.as_f64().unwrap_or(0.0) as f32)?,
      opacity: parse_keyframes(&transform_data["o"], |v| v.as_f64().unwrap_or(100.0) as f32)?,
  })
}

fn parse_keyframes<T, F>(keyframe_data: &Value, value_parser: F) -> Result<Keyframes<T>, Box<dyn std::error::Error>>
where
    F: Fn(&Value) -> T,
    T: Clone,
{
    if let Some(k) = keyframe_data["k"].as_array() {
        if k.is_empty() {
            return Ok(Keyframes::Static(value_parser(&keyframe_data["k"])));
        }
        let mut keyframes = Vec::new();
        for kf in k {
            let time = kf["t"].as_f64().unwrap_or(0.0) as f32;
            let value = value_parser(&kf["s"]);
            keyframes.push(Keyframe { time, value });
        }
        Ok(Keyframes::Animated(keyframes))
    } else {
        Ok(Keyframes::Static(value_parser(&keyframe_data["k"])))
    }
}

fn composite_layer(
  base_image: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
  layer_image: &DynamicImage,
  layer: &Layer,
  frame: f32,
) -> Result<(), Box<dyn std::error::Error>> {
  let position = interpolate_keyframes(&layer.transform.position, frame);
  let scale = interpolate_keyframes(&layer.transform.scale, frame);
  let rotation = interpolate_keyframes(&layer.transform.rotation, frame);
  let opacity = interpolate_keyframes(&layer.transform.opacity, frame) / 100.0;

  let (width, height) = layer_image.dimensions();
  let scaled_width = (width as f32 * scale.x / 100.0) as u32;
  let scaled_height = (height as f32 * scale.y / 100.0) as u32;

  let scaled_image = image::imageops::resize(layer_image, scaled_width, scaled_height, image::imageops::FilterType::Lanczos3);
  
  // Note: We're not implementing rotation here as it's more complex and requires additional libraries
  // For a full implementation, consider using a library like `imageproc` for rotation

  image::imageops::overlay(base_image, &scaled_image, position.x as i64, position.y as i64);

  Ok(())
}

fn interpolate_keyframes<T: Clone + Add<Output = T> + Mul<f32, Output = T>>(
  keyframes: &Keyframes<T>,
  frame: f32,
) -> T {
  match keyframes {
      Keyframes::Static(value) => value.clone(),
      Keyframes::Animated(kfs) => {
          let mut prev_kf = &kfs[0];
          for kf in kfs.iter().skip(1) {
              if kf.time > frame {
                  let t = (frame - prev_kf.time) / (kf.time - prev_kf.time);
                  return prev_kf.value.clone() * (1.0 - t) + kf.value.clone() * t;
              }
              prev_kf = kf;
          }
          prev_kf.value.clone()
      }
  }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let json_path = "assets/bodymovin.json";
    let assets_path = "assets/images";
    let frame_number = 600; // Change this to the desired frame number
    let output_path = "bodymovin.png";

    let animation_data = load_bodymovin_json(json_path)?;
    let rendered_image = render_frame(&animation_data, assets_path, frame_number)?;
    rendered_image.save(output_path)?;

    println!("Frame {} rendered and saved to {}", frame_number, output_path);
    Ok(())
}
