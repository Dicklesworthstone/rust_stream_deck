# Test Fixtures

Test data for the Stream Deck CLI test suite.

## Regenerating Fixtures

Run the fixture generator test:

```bash
cargo test --test generate_fixtures -- --ignored --nocapture
```

## Directory Structure

### images/valid/
Valid test images in various sizes and formats:

| File | Size | Purpose |
|------|------|---------|
| `exact-72x72.png` | 72x72 | Exact size for Stream Deck XL keys |
| `exact-96x96.png` | 96x96 | Exact size for Stream Deck MK2 keys |
| `large-256x256.png` | 256x256 | Requires downscaling |
| `large-1024x1024.png` | 1024x1024 | Very large, tests memory handling |
| `small-50x50.png` | 50x50 | Requires upscaling |
| `nonsquare-100x80.png` | 100x80 | Non-square aspect ratio |
| `transparent.png` | 72x72 | Image with alpha channel |
| `grayscale.png` | 72x72 | Grayscale gradient |

### images/invalid/
Invalid files for error handling tests:

| File | Issue |
|------|-------|
| `empty.png` | Zero bytes |
| `not-image.txt.png` | Text file with .png extension |
| `truncated.png` | Valid PNG header, incomplete data |
| `corrupted.png` | Corrupted PNG header |
| `fake-header.bin` | Valid IHDR, garbage data |

### images/batch/
Batch operation test sets:

| Directory | Contents | Purpose |
|-----------|----------|---------|
| `complete-32/` | key-0.png to key-31.png | Full Stream Deck XL layout |
| `complete-15/` | key-0.png to key-14.png | Full Stream Deck MK2 layout |
| `complete-6/` | key-0.png to key-5.png | Full Stream Deck Mini layout |
| `partial-10/` | Every 3rd key (0,3,6...) | Sparse layout testing |
| `gaps/` | Even numbers only (0,2,4...) | Gap handling tests |
| `custom-pattern/` | icon_00.png to icon_07.png | Custom naming pattern |
| `mixed-formats/` | (empty) | Reserved for format mixing tests |

### images/colors/
Solid color test images (72x72):

- `red.png` - #FF0000
- `green.png` - #00FF00
- `blue.png` - #0000FF
- `white.png` - #FFFFFF
- `black.png` - #000000
- `yellow.png` - #FFFF00
- `cyan.png` - #00FFFF
- `magenta.png` - #FF00FF

## Usage in Tests

```rust
use crate::common::fixtures::fixtures_path;

#[test]
fn test_with_fixtures() {
    let valid_image = fixtures_path("images/valid/exact-72x72.png");
    let batch_dir = fixtures_path("images/batch/complete-32");
    // ...
}
```
