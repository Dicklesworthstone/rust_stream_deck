//! Integration tests for image processing operations.
//!
//! Tests verify image loading, resizing, and error handling
//! using the test fixture images.

use std::path::PathBuf;

use image::GenericImageView;
use sd::error::SdError;
use sd::image_ops::{ResizeStrategy, load_and_resize};

/// Get the path to test fixtures directory.
fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("images")
}

/// Test loading a valid 72x72 PNG image.
#[test]
fn test_load_exact_72x72() {
    let path = fixtures_dir().join("valid").join("exact-72x72.png");
    if !path.exists() {
        eprintln!("Skipping test: fixture not found at {:?}", path);
        return;
    }

    let img = load_and_resize(&path, 72, 72, ResizeStrategy::Fit).unwrap();
    let (w, h) = img.dimensions();
    assert_eq!(w, 72);
    assert_eq!(h, 72);
}

/// Test loading a 96x96 image.
#[test]
fn test_load_exact_96x96() {
    let path = fixtures_dir().join("valid").join("exact-96x96.png");
    if !path.exists() {
        eprintln!("Skipping test: fixture not found at {:?}", path);
        return;
    }

    let img = load_and_resize(&path, 96, 96, ResizeStrategy::Fit).unwrap();
    let (w, h) = img.dimensions();
    assert_eq!(w, 96);
    assert_eq!(h, 96);
}

/// Test fit strategy with larger image.
#[test]
fn test_resize_fit_strategy() {
    let path = fixtures_dir().join("valid").join("large-256x256.png");
    if !path.exists() {
        eprintln!("Skipping test: fixture not found at {:?}", path);
        return;
    }

    let img = load_and_resize(&path, 72, 72, ResizeStrategy::Fit).unwrap();
    let (w, h) = img.dimensions();

    // Fit should produce exact target dimensions (letterboxed)
    assert_eq!(w, 72);
    assert_eq!(h, 72);
}

/// Test fill strategy with larger image.
#[test]
fn test_resize_fill_strategy() {
    let path = fixtures_dir().join("valid").join("large-256x256.png");
    if !path.exists() {
        eprintln!("Skipping test: fixture not found at {:?}", path);
        return;
    }

    let img = load_and_resize(&path, 72, 72, ResizeStrategy::Fill).unwrap();
    let (w, h) = img.dimensions();

    // Fill should produce exact target dimensions (cropped)
    assert_eq!(w, 72);
    assert_eq!(h, 72);
}

/// Test stretch strategy.
#[test]
fn test_resize_stretch_strategy() {
    let path = fixtures_dir().join("valid").join("large-256x256.png");
    if !path.exists() {
        eprintln!("Skipping test: fixture not found at {:?}", path);
        return;
    }

    let img = load_and_resize(&path, 72, 72, ResizeStrategy::Stretch).unwrap();
    let (w, h) = img.dimensions();

    // Stretch should produce exact target dimensions
    assert_eq!(w, 72);
    assert_eq!(h, 72);
}

/// Test loading a very large image.
#[test]
fn test_load_large_image() {
    let path = fixtures_dir().join("valid").join("large-1024x1024.png");
    if !path.exists() {
        eprintln!("Skipping test: fixture not found at {:?}", path);
        return;
    }

    let img = load_and_resize(&path, 96, 96, ResizeStrategy::Fit).unwrap();
    let (w, h) = img.dimensions();
    assert_eq!(w, 96);
    assert_eq!(h, 96);
}

/// Test loading a small image that needs upscaling.
#[test]
fn test_load_small_image_upscale() {
    let path = fixtures_dir().join("valid").join("small-50x50.png");
    if !path.exists() {
        eprintln!("Skipping test: fixture not found at {:?}", path);
        return;
    }

    let img = load_and_resize(&path, 72, 72, ResizeStrategy::Fit).unwrap();
    let (w, h) = img.dimensions();
    assert_eq!(w, 72);
    assert_eq!(h, 72);
}

/// Test loading non-square image.
#[test]
fn test_load_nonsquare_image() {
    let path = fixtures_dir().join("valid").join("nonsquare-100x80.png");
    if !path.exists() {
        eprintln!("Skipping test: fixture not found at {:?}", path);
        return;
    }

    // Fit should handle non-square correctly
    let img = load_and_resize(&path, 72, 72, ResizeStrategy::Fit).unwrap();
    let (w, h) = img.dimensions();
    assert_eq!(w, 72);
    assert_eq!(h, 72);
}

/// Test loading grayscale image.
#[test]
fn test_load_grayscale_image() {
    let path = fixtures_dir().join("valid").join("grayscale.png");
    if !path.exists() {
        eprintln!("Skipping test: fixture not found at {:?}", path);
        return;
    }

    let img = load_and_resize(&path, 72, 72, ResizeStrategy::Fit).unwrap();
    let (w, h) = img.dimensions();
    assert_eq!(w, 72);
    assert_eq!(h, 72);
}

/// Test loading image with transparency.
#[test]
fn test_load_transparent_image() {
    let path = fixtures_dir().join("valid").join("transparent.png");
    if !path.exists() {
        eprintln!("Skipping test: fixture not found at {:?}", path);
        return;
    }

    let img = load_and_resize(&path, 72, 72, ResizeStrategy::Fit).unwrap();
    let (w, h) = img.dimensions();
    assert_eq!(w, 72);
    assert_eq!(h, 72);
}

/// Test error on non-existent file.
#[test]
fn test_load_nonexistent_file() {
    let path = PathBuf::from("/nonexistent/path/image.png");
    let result = load_and_resize(&path, 72, 72, ResizeStrategy::Fit);

    assert!(matches!(result, Err(SdError::ImageNotFound { .. })));
}

/// Test error on empty file.
#[test]
fn test_load_empty_file() {
    let path = fixtures_dir().join("invalid").join("empty.png");
    if !path.exists() {
        eprintln!("Skipping test: fixture not found at {:?}", path);
        return;
    }

    let result = load_and_resize(&path, 72, 72, ResizeStrategy::Fit);
    assert!(result.is_err());
}

/// Test error on corrupted file.
#[test]
fn test_load_corrupted_file() {
    let path = fixtures_dir().join("invalid").join("corrupted.png");
    if !path.exists() {
        eprintln!("Skipping test: fixture not found at {:?}", path);
        return;
    }

    let result = load_and_resize(&path, 72, 72, ResizeStrategy::Fit);
    assert!(result.is_err());
}

/// Test error on truncated file.
#[test]
fn test_load_truncated_file() {
    let path = fixtures_dir().join("invalid").join("truncated.png");
    if !path.exists() {
        eprintln!("Skipping test: fixture not found at {:?}", path);
        return;
    }

    let result = load_and_resize(&path, 72, 72, ResizeStrategy::Fit);
    assert!(result.is_err());
}

/// Test error on non-image file with .png extension.
#[test]
fn test_load_fake_png() {
    let path = fixtures_dir().join("invalid").join("not-image.txt.png");
    if !path.exists() {
        eprintln!("Skipping test: fixture not found at {:?}", path);
        return;
    }

    let result = load_and_resize(&path, 72, 72, ResizeStrategy::Fit);
    assert!(result.is_err());
}

/// Test batch fixture images (complete-32 set).
#[test]
fn test_batch_complete_32() {
    let batch_dir = fixtures_dir().join("batch").join("complete-32");
    if !batch_dir.exists() {
        eprintln!("Skipping test: batch fixtures not found at {:?}", batch_dir);
        return;
    }

    // Test loading all 32 keys
    for key in 0..32 {
        let path = batch_dir.join(format!("key-{}.png", key));
        if path.exists() {
            let img = load_and_resize(&path, 96, 96, ResizeStrategy::Fit).unwrap();
            let (w, h) = img.dimensions();
            assert_eq!(w, 96);
            assert_eq!(h, 96);
        }
    }
}

/// Test batch fixture images (complete-6 for Mini).
#[test]
fn test_batch_complete_6() {
    let batch_dir = fixtures_dir().join("batch").join("complete-6");
    if !batch_dir.exists() {
        eprintln!("Skipping test: batch fixtures not found at {:?}", batch_dir);
        return;
    }

    // Test loading all 6 keys
    for key in 0..6 {
        let path = batch_dir.join(format!("key-{}.png", key));
        if path.exists() {
            let img = load_and_resize(&path, 72, 72, ResizeStrategy::Fit).unwrap();
            let (w, h) = img.dimensions();
            assert_eq!(w, 72);
            assert_eq!(h, 72);
        }
    }
}

/// Test color fixture images.
#[test]
fn test_color_fixtures() {
    let color_dir = fixtures_dir().join("colors");
    if !color_dir.exists() {
        eprintln!("Skipping test: color fixtures not found at {:?}", color_dir);
        return;
    }

    let colors = ["red", "green", "blue", "yellow", "cyan", "magenta", "white", "black"];
    for color in colors {
        let path = color_dir.join(format!("{}.png", color));
        if path.exists() {
            let img = load_and_resize(&path, 72, 72, ResizeStrategy::Fit).unwrap();
            let (w, h) = img.dimensions();
            assert_eq!(w, 72);
            assert_eq!(h, 72);
        }
    }
}

/// Test different target sizes.
#[test]
fn test_various_target_sizes() {
    let path = fixtures_dir().join("valid").join("exact-72x72.png");
    if !path.exists() {
        eprintln!("Skipping test: fixture not found at {:?}", path);
        return;
    }

    // Stream Deck Mini (72x72)
    let img = load_and_resize(&path, 72, 72, ResizeStrategy::Fit).unwrap();
    assert_eq!(img.dimensions(), (72, 72));

    // Stream Deck XL (96x96)
    let img = load_and_resize(&path, 96, 96, ResizeStrategy::Fit).unwrap();
    assert_eq!(img.dimensions(), (96, 96));

    // Stream Deck Plus (120x120)
    let img = load_and_resize(&path, 120, 120, ResizeStrategy::Fit).unwrap();
    assert_eq!(img.dimensions(), (120, 120));
}
