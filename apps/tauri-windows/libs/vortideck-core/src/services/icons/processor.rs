use base64::decode;
use image::DynamicImage;

/// Handles image processing operations (Base64 → WebP conversion, resizing)
pub struct IconProcessor {
    target_size: u32,
}

impl IconProcessor {
    pub fn new() -> Self {
        Self {
            target_size: 120, // Default 120x120 pixels
        }
    }

    pub fn with_size(target_size: u32) -> Self {
        Self { target_size }
    }

    /// Process Base64 image data to optimal format (WebP or PNG, whichever is smaller)
    pub async fn process_base64_to_webp(&self, base64_image: &str) -> Result<Vec<u8>, String> {
        let base64_image = base64_image.to_string();
        let target_size = self.target_size;

        // Offload to background thread for CPU-intensive work
        tokio::task::spawn_blocking(move || {
            use image::{imageops::FilterType, GenericImageView, ImageFormat};
            use std::io::Cursor;

            // Decode Base64 data
            let image_bytes =
                decode(&base64_image).map_err(|e| format!("Error decoding Base64 data: {}", e))?;

            // Load image from bytes
            let img = image::load_from_memory(&image_bytes)
                .map_err(|e| format!("Error loading image from bytes: {}", e))?;

            let (w, h) = img.dimensions();

            // Quick complexity check: count unique colors in a sample
            // Pixel art/simple graphics typically have <64 colors
            let is_simple_graphic = {
                use std::collections::HashSet;
                let mut colors = HashSet::new();
                for pixel in img.to_rgba8().pixels().take(1000) {
                    // Sample 1000 pixels
                    colors.insert((pixel[0], pixel[1], pixel[2]));
                    if colors.len() > 64 {
                        break;
                    }
                }
                colors.len() <= 64 && w <= 512 && h <= 512
            };

            // Resize with appropriate filter:
            // - Nearest: preserves sharp pixel art edges (Minecraft textures, icons)
            // - Lanczos3: smooth scaling for photos/complex images
            let resized_img = img.resize(
                target_size,
                target_size,
                if is_simple_graphic {
                    FilterType::Nearest
                } else {
                    FilterType::Lanczos3
                },
            );

            // Try multiple compression strategies and keep the smallest result
            // This handles all cases: indexed PNGs, pixel art, photos, complex graphics

            // Try lossy WebP with quality 80 (good for photos/complex images)
            let lossy_webp = webp::Encoder::from_image(&resized_img)
                .map_err(|e| format!("Error creating lossy encoder: {}", e))?
                .encode(80.0);

            // Try lossless WebP (good for simple icons/logos)
            let lossless_webp = webp::Encoder::from_image(&resized_img)
                .map_err(|e| format!("Error creating lossless encoder: {}", e))?
                .encode_lossless();

            // Try PNG (best for indexed/palette images like pixel art, Minecraft textures)
            let mut png_bytes = Vec::new();
            resized_img
                .write_to(&mut Cursor::new(&mut png_bytes), ImageFormat::Png)
                .map_err(|e| format!("Error encoding PNG: {}", e))?;

            // Compare all three and return the smallest
            let mut smallest = lossy_webp.to_vec();
            let mut smallest_size = smallest.len();

            if lossless_webp.len() < smallest_size {
                smallest = lossless_webp.to_vec();
                smallest_size = smallest.len();
            }

            if png_bytes.len() < smallest_size {
                smallest = png_bytes;
            }

            Ok(smallest)
        })
        .await
        .map_err(|e| format!("Error executing image processing task: {}", e))?
    }

    /// Get supported input formats
    pub fn supported_formats() -> Vec<&'static str> {
        vec!["png", "jpg", "jpeg", "gif", "webp", "bmp"]
    }

    /// Validate if input is valid Base64 image data
    pub fn validate_base64_image(base64_data: &str) -> Result<(), String> {
        // Basic validation - check if it's valid base64
        decode(base64_data).map_err(|_| "Invalid Base64 data".to_string())?;

        // Additional validation could check image headers here
        Ok(())
    }
}

impl Default for IconProcessor {
    fn default() -> Self {
        Self::new()
    }
}
