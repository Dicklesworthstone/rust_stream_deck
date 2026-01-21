//! Integration test that generates test fixtures.
//!
//! Run with: cargo test --test generate_fixtures -- --ignored
//!
//! This will populate the tests/fixtures/ directory with all needed test data.

use image::{Rgb, RgbImage, Rgba, RgbaImage};
use std::fs;
use std::path::Path;

#[test]
#[ignore] // Only run manually to regenerate fixtures
fn generate_all_fixtures() {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");

    // Create directory structure
    create_dirs(&base);

    // Generate images
    generate_valid_images(&base);
    generate_invalid_images(&base);
    generate_batch_images(&base);
    generate_color_images(&base);

    println!("Fixtures generated successfully at {:?}", base);
}

fn create_dirs(base: &Path) {
    let dirs = [
        "images/valid",
        "images/invalid",
        "images/batch/complete-32",
        "images/batch/complete-15",
        "images/batch/complete-6",
        "images/batch/partial-10",
        "images/batch/gaps",
        "images/batch/mixed-formats",
        "images/batch/custom-pattern",
        "images/colors",
    ];

    for dir in dirs {
        fs::create_dir_all(base.join(dir)).expect("Failed to create directory");
    }
}

fn generate_valid_images(base: &Path) {
    let valid_dir = base.join("images/valid");

    // Exact sizes for different Stream Deck models
    create_solid_image(&valid_dir.join("exact-72x72.png"), 72, 72, [100, 150, 200]);
    create_solid_image(&valid_dir.join("exact-96x96.png"), 96, 96, [150, 100, 200]);

    // Various sizes requiring resize
    create_solid_image(
        &valid_dir.join("large-256x256.png"),
        256,
        256,
        [200, 100, 150],
    );
    create_solid_image(
        &valid_dir.join("large-1024x1024.png"),
        1024,
        1024,
        [50, 100, 150],
    );
    create_solid_image(&valid_dir.join("small-50x50.png"), 50, 50, [100, 200, 100]);
    create_solid_image(
        &valid_dir.join("nonsquare-100x80.png"),
        100,
        80,
        [200, 200, 100],
    );

    // With transparency
    create_transparent_image(&valid_dir.join("transparent.png"), 72, 72);

    // Grayscale
    create_grayscale_image(&valid_dir.join("grayscale.png"), 72, 72);
}

fn generate_invalid_images(base: &Path) {
    let invalid_dir = base.join("images/invalid");

    // Empty file
    fs::write(invalid_dir.join("empty.png"), b"").expect("Failed to write empty file");

    // Text file with .png extension
    fs::write(
        invalid_dir.join("not-image.txt.png"),
        b"This is not an image file",
    )
    .expect("Failed to write text file");

    // Truncated PNG (valid header, incomplete data)
    #[rustfmt::skip]
    let truncated: [u8; 29] = [
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, // IHDR length
        0x49, 0x48, 0x44, 0x52, // IHDR
        0x00, 0x00, 0x00, 0x48, // width = 72
        0x00, 0x00, 0x00, 0x48, // height = 72
        0x08, 0x02, // bit depth = 8, color type = RGB
        0x00, 0x00, 0x00, // compression, filter, interlace
        // Missing CRC and IDAT chunks
    ];
    fs::write(invalid_dir.join("truncated.png"), &truncated)
        .expect("Failed to write truncated file");

    // Corrupted PNG header
    #[rustfmt::skip]
    let corrupted: [u8; 8] = [
        0x89, 0x50, 0x4E, 0x47, 0xFF, 0xFF, 0xFF, 0xFF, // Bad signature
    ];
    fs::write(invalid_dir.join("corrupted.png"), &corrupted)
        .expect("Failed to write corrupted file");

    // Valid PNG header but invalid data
    #[rustfmt::skip]
    let fake_header: [u8; 37] = [
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, // IHDR length
        0x49, 0x48, 0x44, 0x52, // IHDR
        0x00, 0x00, 0x00, 0x01, // width
        0x00, 0x00, 0x00, 0x01, // height
        0x08, 0x02, 0x00, 0x00, 0x00, // bit depth, color, compression, filter, interlace
        0x90, 0x77, 0x53, 0xDE, // CRC
        0xFF, 0xFF, 0xFF, 0xFF, // Garbage after valid IHDR
    ];
    fs::write(invalid_dir.join("fake-header.bin"), &fake_header)
        .expect("Failed to write fake header file");
}

fn generate_batch_images(base: &Path) {
    let batch_dir = base.join("images/batch");

    // Complete 32-key layout (Stream Deck XL)
    for i in 0..32_u8 {
        let color = key_color(i);
        create_numbered_image(
            &batch_dir.join(format!("complete-32/key-{i}.png")),
            72,
            72,
            color,
            i,
        );
    }

    // Complete 15-key layout (Stream Deck MK2)
    for i in 0..15_u8 {
        let color = key_color(i);
        create_numbered_image(
            &batch_dir.join(format!("complete-15/key-{i}.png")),
            72,
            72,
            color,
            i,
        );
    }

    // Complete 6-key layout (Stream Deck Mini)
    for i in 0..6_u8 {
        let color = key_color(i);
        create_numbered_image(
            &batch_dir.join(format!("complete-6/key-{i}.png")),
            72,
            72,
            color,
            i,
        );
    }

    // Partial layout (every 3rd key)
    for i in (0..32_u8).step_by(3) {
        let color = key_color(i);
        create_numbered_image(
            &batch_dir.join(format!("partial-10/key-{i}.png")),
            72,
            72,
            color,
            i,
        );
    }

    // Gaps (even numbers only)
    for i in (0..16_u8).step_by(2) {
        let color = key_color(i);
        create_numbered_image(&batch_dir.join(format!("gaps/key-{i}.png")), 72, 72, color, i);
    }

    // Custom pattern naming
    for i in 0..8_u8 {
        let color = key_color(i);
        create_numbered_image(
            &batch_dir.join(format!("custom-pattern/icon_{i:02}.png")),
            72,
            72,
            color,
            i,
        );
    }
}

fn generate_color_images(base: &Path) {
    let color_dir = base.join("images/colors");

    create_solid_image(&color_dir.join("red.png"), 72, 72, [255, 0, 0]);
    create_solid_image(&color_dir.join("green.png"), 72, 72, [0, 255, 0]);
    create_solid_image(&color_dir.join("blue.png"), 72, 72, [0, 0, 255]);
    create_solid_image(&color_dir.join("white.png"), 72, 72, [255, 255, 255]);
    create_solid_image(&color_dir.join("black.png"), 72, 72, [0, 0, 0]);
    create_solid_image(&color_dir.join("yellow.png"), 72, 72, [255, 255, 0]);
    create_solid_image(&color_dir.join("cyan.png"), 72, 72, [0, 255, 255]);
    create_solid_image(&color_dir.join("magenta.png"), 72, 72, [255, 0, 255]);
}

fn create_solid_image(path: &Path, width: u32, height: u32, color: [u8; 3]) {
    let img = RgbImage::from_pixel(width, height, Rgb(color));
    img.save(path).expect("Failed to save image");
}

fn create_transparent_image(path: &Path, width: u32, height: u32) {
    let mut img = RgbaImage::new(width, height);
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        // Checkerboard pattern with varying alpha
        let is_dark = (x / 8 + y / 8) % 2 == 0;
        let alpha = if is_dark { 255 } else { 128 };
        *pixel = Rgba([100, 150, 200, alpha]);
    }
    img.save(path).expect("Failed to save image");
}

fn create_grayscale_image(path: &Path, width: u32, height: u32) {
    let mut img = RgbImage::new(width, height);
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        // Gradient from black to white
        #[allow(clippy::cast_possible_truncation)]
        #[allow(clippy::cast_sign_loss)]
        let gray = ((x as f32 / width as f32) * 255.0) as u8;
        *pixel = Rgb([gray, gray, gray]);
    }
    img.save(path).expect("Failed to save image");
}

fn create_numbered_image(path: &Path, width: u32, height: u32, color: [u8; 3], number: u8) {
    let mut img = RgbImage::from_pixel(width, height, Rgb(color));

    // Draw a simple pattern that encodes the number
    // This creates a visual indicator without needing font rendering
    let center_x = width / 2;
    let center_y = height / 2;

    // Draw a dot pattern representing the number
    let dot_size = 4;
    for i in 0..=number {
        #[allow(clippy::cast_possible_truncation)]
        let angle = (f32::from(i) * 360.0 / 32.0).to_radians();
        let radius = 20.0;
        #[allow(clippy::cast_possible_truncation)]
        let dx = (angle.cos() * radius) as i32;
        #[allow(clippy::cast_possible_truncation)]
        let dy = (angle.sin() * radius) as i32;

        #[allow(clippy::cast_sign_loss)]
        let px = (i32::try_from(center_x).unwrap_or(0) + dx).clamp(0, i32::try_from(width).unwrap_or(0) - 1)
            as u32;
        #[allow(clippy::cast_sign_loss)]
        let py = (i32::try_from(center_y).unwrap_or(0) + dy).clamp(0, i32::try_from(height).unwrap_or(0) - 1)
            as u32;

        // Draw small dot
        for ox in 0..dot_size {
            for oy in 0..dot_size {
                let x = (px + ox).min(width - 1);
                let y = (py + oy).min(height - 1);
                img.put_pixel(x, y, Rgb([255, 255, 255]));
            }
        }
    }

    img.save(path).expect("Failed to save image");
}

fn key_color(index: u8) -> [u8; 3] {
    // Generate distinct colors for each key
    let hue = (f32::from(index) * 360.0 / 32.0) % 360.0;
    hsv_to_rgb(hue, 0.7, 0.8)
}

#[allow(clippy::cast_possible_truncation)]
#[allow(clippy::cast_sign_loss)]
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [u8; 3] {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r, g, b) = match h as u32 {
        0..60 => (c, x, 0.0),
        60..120 => (x, c, 0.0),
        120..180 => (0.0, c, x),
        180..240 => (0.0, x, c),
        240..300 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    [
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    ]
}
