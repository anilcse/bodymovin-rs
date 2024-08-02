use image::{ImageBuffer, Rgba, DynamicImage, GenericImageView};
use serde_json::Value;
use std::fs::{File, create_dir_all};
use std::io::BufReader;
use std::collections::HashMap;
use std::ops::{Add, Mul};
// use num_rational::Rational64 as Rational;
use rayon::prelude::*;
use std::sync::Arc;
use std::time::Instant;
use std::error::Error;
use std::fmt;
use imageproc::geometric_transformations::rotate_about_center;

// Define a custom error type
#[derive(Debug)]
enum LottieError {
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
    fn new(x: f32, y: f32) -> Self { Vec2 { x, y } }
}

impl Add for Vec2 {
    type Output = Self;
    fn add(self, other: Self) -> Self { Vec2::new(self.x + other.x, self.y + other.y) }
}

impl Mul<f32> for Vec2 {
    type Output = Self;
    fn mul(self, scalar: f32) -> Self { Vec2::new(self.x * scalar, self.y * scalar) }
}
struct Layer {
    start_frame: f32,
    end_frame: f32,
    transform: Transform,
    asset_id: Option<String>,
}

struct Asset {
    image: Arc<DynamicImage>,
}


#[derive(Clone)]
struct Transform {
    position: Vec2,
    scale: Vec2,
    opacity: f32,
    rotation: f32,
}

impl fmt::Display for LottieError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LottieError::IoError(err) => write!(f, "IO error: {}", err),
            LottieError::ImageError(err) => write!(f, "Image error: {}", err),
            LottieError::JsonError(err) => write!(f, "JSON error: {}", err),
            LottieError::OtherError(err) => write!(f, "Error: {}", err),
        }
    }
}

impl Error for LottieError {}

impl From<std::io::Error> for LottieError {
    fn from(err: std::io::Error) -> Self {
        LottieError::IoError(err)
    }
}

impl From<image::ImageError> for LottieError {
    fn from(err: image::ImageError) -> Self {
        LottieError::ImageError(err)
    }
}

impl From<serde_json::Error> for LottieError {
    fn from(err: serde_json::Error) -> Self {
        LottieError::JsonError(err)
    }
}


// Update function signatures to use LottieError
fn load_bodymovin_json(path: &str) -> Result<Value, LottieError> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    Ok(serde_json::from_reader(reader)?)
}

fn load_assets(assets_path: &str, animation_data: &Value) -> Result<HashMap<String, Asset>, LottieError> {
    let mut assets = HashMap::new();
    if let Some(assets_array) = animation_data["assets"].as_array() {
        for asset in assets_array {
            let id = asset["id"].as_str().ok_or_else(|| LottieError::OtherError("Missing asset id".to_string()))?;
            let path = format!("{}/{}", assets_path, asset["p"].as_str().ok_or_else(|| LottieError::OtherError("Missing asset path".to_string()))?);
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
    frame_number: u32
) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut image = ImageBuffer::new(width, height);

    for layer in layers.iter().rev() {
        if frame_number as f32 >= layer.start_frame && frame_number as f32 <= layer.end_frame {
            if let Some(asset_id) = &layer.asset_id {
                if let Some(asset) = assets.get(asset_id) {
                    composite_layer(&mut image, &asset.image, &layer.transform);
                }
            }
        }
    }

    image
}


fn composite_layer(
    base_image: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    layer_image: &DynamicImage,
    transform: &Transform,
) {
    let (width, height) = layer_image.dimensions();
    let scaled_width = (width as f32 * transform.scale.x) as u32;
    let scaled_height = (height as f32 * transform.scale.y) as u32;

    //println!("w:h {}x{} sw:sh: {}x{} -- {}x{}",width, height, scaled_width, scaled_height, transform.scale.x, transform.scale.y);

    let mut resized_image = layer_image.resize(scaled_width, scaled_height, image::imageops::FilterType::Lanczos3);

    // Apply rotation
    if transform.rotation != 0.0 {
        resized_image = DynamicImage::ImageRgba8(rotate_about_center(
            &resized_image.to_rgba8(),
            transform.rotation.to_radians(),
            imageproc::geometric_transformations::Interpolation::Bilinear,
            Rgba([0, 0, 0, 0])
        ));
    }

    // Calculate position
    let x = transform.position.x - (scaled_width as f32 / 2.0);
    let y = transform.position.y - (scaled_height as f32 / 2.0);

    // Overlay the image
    image::imageops::overlay(base_image, &resized_image, x as i64, y as i64);

    // Apply opacity
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

fn parse_scale(v: &Value) -> Vec2 {
    let scale = if v.is_array() {
        Vec2 {
            x: v[0].as_f64().unwrap_or(100.0) as f32,
            y: v[1].as_f64().unwrap_or(100.0) as f32,
        }
    } else if v.is_object() {
        Vec2 {
            x: v["x"].as_f64().unwrap_or(100.0) as f32,
            y: v["y"].as_f64().unwrap_or(100.0) as f32,
        }
    } else {
        Vec2 { x: 100.0, y: 100.0 }
    };
    
    Vec2 {
        x: scale.x / 100.0,
        y: scale.y / 100.0,
    }
}

fn parse_layers(animation_data: &Value) -> Result<Vec<Layer>, LottieError> {
    animation_data["layers"].as_array()
        .ok_or_else(|| LottieError::OtherError("No layers found".to_string()))?
        .iter()
        .map(|layer| {
            Ok(Layer {
                start_frame: layer["ip"].as_f64().unwrap_or(0.0) as f32,
                end_frame: layer["op"].as_f64().unwrap_or(0.0) as f32,
                transform: parse_transform(&layer["ks"])?,
                asset_id: layer["refId"].as_str().map(String::from),
            })
        })
        .collect()
}


fn parse_transform(transform_data: &Value) -> Result<Transform, LottieError> {
    //println!("Raw transform data: {:?}", transform_data);

    let position = if transform_data["p"].is_object() {
        parse_vec2(&transform_data["p"]["k"])
    } else {
        parse_vec2(&transform_data["p"])
    };

    let scale = if transform_data["s"].is_object() {
        parse_scale(&transform_data["s"]["k"])
    } else {
        parse_scale(&transform_data["s"])
    };

    let rotation = if transform_data["r"].is_object() {
        transform_data["r"]["k"].as_f64().unwrap_or(0.0) as f32
    } else {
        transform_data["r"].as_f64().unwrap_or(0.0) as f32
    };

    let opacity = if transform_data["o"].is_object() {
        transform_data["o"]["k"].as_f64().unwrap_or(100.0) as f32
    } else {
        transform_data["o"].as_f64().unwrap_or(100.0) as f32
    };

    // Ensure opacity is between 0 and 1
    let opacity = (opacity / 100.0).clamp(0.0, 1.0);

    //println!("Parsed transform: position: {:?}, scale: {:?}, rotation: {}, opacity: {}", position, scale, rotation, opacity);

    Ok(Transform {
        position,
        scale,
        rotation,
        opacity,
    })
}

fn save_frame(_frame: &ImageBuffer<Rgba<u8>, Vec<u8>>, _output_dir: &str, _frame_number: u32) -> Result<(), LottieError> {
    // let file_name = format!("frame_{:04}.png", frame_number);
    // let path = Path::new(output_dir).join(file_name);
    // frame.save(path)?;
    Ok(())
}

fn main() -> Result<(), LottieError> {
    let start = Instant::now();

    let json_path = "assets/bodymovin.json";
    let assets_path = "assets/images";
    let output_directory = "output_frames";

    // Create output directory if it doesn't exist
    create_dir_all(output_directory)?;

    let animation_data = load_bodymovin_json(json_path)?;
    let assets = load_assets(assets_path, &animation_data)?;
    let layers = parse_layers(&animation_data)?;

    let width = animation_data["w"].as_u64().unwrap_or(540) as u32;
    let height = animation_data["h"].as_u64().unwrap_or(800) as u32;
    let total_frames = animation_data["op"].as_f64().unwrap_or(0.0) as u32;
    let frame_rate = animation_data["fr"].as_f64().unwrap_or(30.0);

    //println!("Rendering and saving {} frames...", total_frames);

    // Render and save frames in parallel
    (0..total_frames).into_par_iter().try_for_each(|frame_number| {
        let frame = render_frame(width, height, &assets, &layers, frame_number);
        save_frame(&frame, output_directory, frame_number)
    })?;

    let elapsed = start.elapsed();
    println!("Rendered and saved {} frames", total_frames);
    println!("Duration: {:.2} seconds", total_frames as f64 / frame_rate);
    println!("Frame rate: {} fps", frame_rate);
    println!("Total rendering and saving time: {:?}", elapsed);
    println!("Average time per frame: {:?}", elapsed / total_frames);

    Ok(())
}