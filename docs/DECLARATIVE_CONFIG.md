# Declarative Config Schema

This document describes the YAML/TOML schema for declarative Stream Deck profiles.
It reflects the current implementation in `src/config/declarative.rs`,
`src/config/key_config.rs`, `src/config/selector.rs`, and `src/config/path.rs`.

## Overview

Declarative configs let you describe desired device state in a file and apply it
via CLI commands (validate/apply). A config file contains:

- Optional profile metadata
- Optional device targeting (serial)
- Optional brightness
- A map of key selectors to key configurations

Supported file formats are **YAML** (`.yaml`, `.yml`) and **TOML** (`.toml`).
The format is inferred from the file extension.

## Top-Level Schema

YAML:

```yaml
name: "Work Profile"          # optional
device: "ABC123"             # optional serial
brightness: 80                # optional (0-100)

keys:
  "0":
    image: "~/icons/chrome.png"

  "8-15":
    pattern: "./icons/row2/{index}.png"
    missing: skip             # optional

  "row-3":
    color: "#222222"

  "default":
    clear: true
```

TOML:

```toml
name = "Work Profile"
device = "ABC123"
brightness = 80

[keys."0"]
image = "~/icons/chrome.png"

[keys."8-15"]
pattern = "./icons/row2/{index}.png"
missing = "skip"

[keys."row-3"]
color = "#222222"

[keys.default]
clear = true
```

### Fields

| Field | Type | Description |
| --- | --- | --- |
| `name` | string | Optional profile name. |
| `device` | string | Optional device serial. If set, config applies only to that device. |
| `brightness` | integer | Optional brightness, 0-100. |
| `keys` | map<string, KeyConfig> | Map of key selector strings to configurations. |

## Key Selectors

Selectors define which keys a config applies to. Supported formats:

- **Single key**: `"0"`, `"15"`
- **Range**: `"8-15"` (inclusive)
- **Row**: `"row-0"`, `"row-3"`
- **Column**: `"col-0"`, `"col-4"`
- **Default**: `"default"` (fallback for unmatched keys)

### Selector Priority (Conflict Resolution)

When multiple selectors match a key, the most specific wins. Priority order:

1. Single key
2. Range
3. Row / Column
4. Default (lowest)

## KeyConfig Variants

### Image

```yaml
"0":
  image: "~/icons/chrome.png"
  label: "Chrome"   # optional, reserved for future
```

- `image` is a path to an image file.
- `label` is optional (reserved for future enhancements).

### Pattern

```yaml
"8-15":
  pattern: "./icons/row2/{index}.png"
  missing: skip
```

- `pattern` **must** include `{index}` placeholder.
- `missing` controls behavior for missing files:
  - `error` (default) – fail validation
  - `skip` – skip missing keys
  - `clear` – clear keys with missing files

### Color

```yaml
"row-3":
  color: "#FF5500"
```

Color formats:

- Hex string: `"#FF5500"` or `"FF5500"`
- RGB array: `[255, 85, 0]`
- Named colors: `black`, `white`, `red`, `green`, `blue`, `yellow`, `cyan`,
  `magenta`, `orange`, `purple`, `pink`, `gray` / `grey`

### Clear

```yaml
"default":
  clear: true
```

- `clear: true` explicitly clears keys (sets to black).
- `clear: false` is invalid (omit the key instead).

## Path Resolution

Paths resolve according to `src/config/path.rs`:

1. Absolute paths are used as-is.
2. Paths starting with `~` are expanded to the user home directory.
3. Relative paths are resolved relative to the config file’s directory.

Supported image extensions: `.png`, `.jpg`, `.jpeg`, `.gif`, `.bmp`, `.webp`.

## Validation Rules

Validation happens during load:

- Brightness must be 0-100.
- Each selector string must parse correctly.
- Each `KeyConfig` must be valid:
  - Image path not empty
  - Pattern must contain `{index}`
  - Color must parse
  - Clear must be `true`

Invalid selectors or configs are rejected with a `ConfigParse` or `ConfigInvalid` error.

## Notes

- Unknown fields are ignored (forward compatibility).
- Device targeting uses **serial** string only (no model filter yet).
- Default selector only applies to keys not matched by other selectors.

