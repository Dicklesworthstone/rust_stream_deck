//! Key selector types for targeting keys in Stream Deck profiles.
//!
//! This module provides the [`KeySelector`] enum which allows users to
//! specify keys using various convenient formats: single keys, ranges,
//! rows, columns, or a default fallback.

use std::str::FromStr;

use serde::{Deserialize, Serialize};
use tracing::{debug, trace, warn};

use crate::device::DeviceInfo;
use crate::error::{Result, SdError};

/// Selector for targeting one or more keys in configuration.
///
/// Supports multiple selection modes:
/// - Single key by index: `"0"`, `"15"`
/// - Inclusive range: `"8-15"`
/// - All keys in a row: `"row-0"`, `"row-3"`
/// - All keys in a column: `"col-0"`, `"col-4"`
/// - Default fallback for unmatched keys: `"default"`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum KeySelector {
    /// Single key by index: "0", "15".
    Single(u8),

    /// Inclusive range of keys: "8-15".
    Range {
        /// Start of range (inclusive).
        start: u8,
        /// End of range (inclusive).
        end: u8,
    },

    /// All keys in a row: "row-0", "row-3".
    Row(u8),

    /// All keys in a column: "col-0", "col-4".
    Column(u8),

    /// Default fallback for keys not matched by other selectors.
    Default,
}

impl KeySelector {
    /// Parse a selector from a string.
    ///
    /// # Supported Formats
    ///
    /// - Single key: `"0"`, `"15"`, `" 5 "` (whitespace trimmed)
    /// - Range: `"8-15"` (inclusive, start must be <= end)
    /// - Row: `"row-0"`, `"row-3"`
    /// - Column: `"col-0"`, `"col-7"`
    /// - Default: `"default"`
    ///
    /// # Errors
    ///
    /// Returns an error if the string doesn't match any valid format.
    pub fn parse(s: &str) -> Result<Self> {
        let s = s.trim();
        trace!(input = %s, "Parsing key selector");

        if s == "default" {
            debug!("Parsed default selector");
            return Ok(Self::Default);
        }

        if let Some(row) = s.strip_prefix("row-") {
            let row_num = row
                .parse::<u8>()
                .map_err(|_| SdError::ConfigParse(format!("Invalid row number: {row}")))?;
            debug!(row = row_num, "Parsed row selector");
            return Ok(Self::Row(row_num));
        }

        if let Some(col) = s.strip_prefix("col-") {
            let col_num = col
                .parse::<u8>()
                .map_err(|_| SdError::ConfigParse(format!("Invalid column number: {col}")))?;
            debug!(col = col_num, "Parsed column selector");
            return Ok(Self::Column(col_num));
        }

        // Check for range (dash not at position 0)
        if let Some(dash_pos) = s.find('-') {
            if dash_pos > 0 {
                let start_str = &s[..dash_pos];
                let end_str = &s[dash_pos + 1..];

                let start = start_str.parse::<u8>().map_err(|_| {
                    SdError::ConfigParse(format!("Invalid range start: {start_str}"))
                })?;
                let end = end_str
                    .parse::<u8>()
                    .map_err(|_| SdError::ConfigParse(format!("Invalid range end: {end_str}")))?;

                if start > end {
                    return Err(SdError::ConfigParse(format!(
                        "Range start ({start}) must be <= end ({end})"
                    )));
                }

                debug!(start, end, "Parsed range selector");
                return Ok(Self::Range { start, end });
            }
        }

        // Try single key index
        let index = s
            .parse::<u8>()
            .map_err(|_| SdError::ConfigParse(format!("Invalid key selector: '{s}'")))?;
        debug!(key = index, "Parsed single key selector");
        Ok(Self::Single(index))
    }

    /// Resolve this selector to concrete key indices for a device.
    ///
    /// # Errors
    ///
    /// Returns an error if the selector targets keys outside the device's range.
    pub fn resolve(&self, device: &DeviceInfo) -> Result<Vec<u8>> {
        trace!(selector = ?self, device_keys = device.key_count, "Resolving selector");

        match self {
            Self::Single(idx) => {
                if *idx >= device.key_count {
                    warn!(
                        key = idx,
                        max = device.key_count,
                        "Single key selector out of range"
                    );
                    return Err(SdError::InvalidKeyIndex {
                        index: *idx,
                        max: device.key_count,
                        max_idx: device.key_count.saturating_sub(1),
                    });
                }
                debug!(keys = ?[*idx], "Resolved single selector");
                Ok(vec![*idx])
            }

            Self::Range { start, end } => {
                if *end >= device.key_count {
                    warn!(
                        end = end,
                        max = device.key_count,
                        "Range end out of device range"
                    );
                    return Err(SdError::InvalidKeyIndex {
                        index: *end,
                        max: device.key_count,
                        max_idx: device.key_count.saturating_sub(1),
                    });
                }
                let keys: Vec<u8> = (*start..=*end).collect();
                debug!(keys = ?keys, "Resolved range selector");
                Ok(keys)
            }

            Self::Row(row) => {
                if *row >= device.rows {
                    warn!(
                        row = row,
                        max_rows = device.rows,
                        "Row selector out of range"
                    );
                    return Err(SdError::ConfigInvalid(format!(
                        "Row {row} out of range (device has {} rows: 0-{})",
                        device.rows,
                        device.rows.saturating_sub(1)
                    )));
                }
                let start = *row * device.cols;
                let end = start + device.cols;
                let keys: Vec<u8> = (start..end).collect();
                debug!(row = row, keys = ?keys, "Resolved row selector");
                Ok(keys)
            }

            Self::Column(col) => {
                if *col >= device.cols {
                    warn!(
                        col = col,
                        max_cols = device.cols,
                        "Column selector out of range"
                    );
                    return Err(SdError::ConfigInvalid(format!(
                        "Column {col} out of range (device has {} columns: 0-{})",
                        device.cols,
                        device.cols.saturating_sub(1)
                    )));
                }
                let keys: Vec<u8> = (0..device.rows)
                    .map(|row| row * device.cols + col)
                    .collect();
                debug!(col = col, keys = ?keys, "Resolved column selector");
                Ok(keys)
            }

            Self::Default => {
                // Return empty - handled specially during config resolution
                debug!("Default selector returns empty (handled specially)");
                Ok(vec![])
            }
        }
    }

    /// Get the priority of this selector for conflict resolution.
    ///
    /// Lower values have higher priority. When multiple selectors match
    /// the same key, the one with lower priority wins.
    ///
    /// Priority order:
    /// 1. Single (0) - most specific
    /// 2. Range (1)
    /// 3. Row/Column (2)
    /// 4. Default (255) - lowest priority
    #[must_use]
    pub const fn priority(&self) -> u8 {
        match self {
            Self::Single(_) => 0,
            Self::Range { .. } => 1,
            Self::Row(_) | Self::Column(_) => 2,
            Self::Default => 255,
        }
    }

    /// Check if this selector might match a given key index.
    ///
    /// Note: For Row, Column, and Default selectors, this is a heuristic
    /// that doesn't account for actual device layout. Use `resolve()` for
    /// accurate matching against a specific device.
    #[must_use]
    pub const fn might_match(&self, key: u8) -> bool {
        match self {
            Self::Single(idx) => *idx == key,
            Self::Range { start, end } => key >= *start && key <= *end,
            Self::Row(_) | Self::Column(_) => true, // Need device info for exact check
            Self::Default => true,
        }
    }
}

impl FromStr for KeySelector {
    type Err = SdError;

    fn from_str(s: &str) -> Result<Self> {
        Self::parse(s)
    }
}

impl<'de> Deserialize<'de> for KeySelector {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::parse(&s).map_err(serde::de::Error::custom)
    }
}

impl Serialize for KeySelector {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = match self {
            Self::Single(idx) => idx.to_string(),
            Self::Range { start, end } => format!("{start}-{end}"),
            Self::Row(row) => format!("row-{row}"),
            Self::Column(col) => format!("col-{col}"),
            Self::Default => "default".to_string(),
        };
        serializer.serialize_str(&s)
    }
}

impl std::fmt::Display for KeySelector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Single(idx) => write!(f, "{idx}"),
            Self::Range { start, end } => write!(f, "{start}-{end}"),
            Self::Row(row) => write!(f, "row-{row}"),
            Self::Column(col) => write!(f, "col-{col}"),
            Self::Default => write!(f, "default"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a test DeviceInfo for XL (32 keys, 8x4).
    fn xl_device() -> DeviceInfo {
        DeviceInfo {
            serial: "TEST-XL".to_string(),
            product_name: "Stream Deck XL".to_string(),
            firmware_version: "1.0.0".to_string(),
            key_count: 32,
            key_width: 96,
            key_height: 96,
            rows: 4,
            cols: 8,
            kind: "xl".to_string(),
        }
    }

    /// Create a test DeviceInfo for Mini (6 keys, 3x2).
    fn mini_device() -> DeviceInfo {
        DeviceInfo {
            serial: "TEST-MINI".to_string(),
            product_name: "Stream Deck Mini".to_string(),
            firmware_version: "1.0.0".to_string(),
            key_count: 6,
            key_width: 72,
            key_height: 72,
            rows: 2,
            cols: 3,
            kind: "mini".to_string(),
        }
    }

    #[test]
    fn test_parse_single() {
        assert_eq!(KeySelector::parse("0").unwrap(), KeySelector::Single(0));
        assert_eq!(KeySelector::parse("15").unwrap(), KeySelector::Single(15));
        assert_eq!(KeySelector::parse(" 5 ").unwrap(), KeySelector::Single(5));
        assert_eq!(KeySelector::parse("255").unwrap(), KeySelector::Single(255));
    }

    #[test]
    fn test_parse_range() {
        assert_eq!(
            KeySelector::parse("8-15").unwrap(),
            KeySelector::Range { start: 8, end: 15 }
        );
        assert_eq!(
            KeySelector::parse("0-0").unwrap(),
            KeySelector::Range { start: 0, end: 0 }
        );
        assert_eq!(
            KeySelector::parse("0-31").unwrap(),
            KeySelector::Range { start: 0, end: 31 }
        );
    }

    #[test]
    fn test_parse_range_invalid() {
        // start > end
        assert!(KeySelector::parse("15-8").is_err());
        // Invalid numbers
        assert!(KeySelector::parse("abc-def").is_err());
        assert!(KeySelector::parse("0-abc").is_err());
    }

    #[test]
    fn test_parse_row() {
        assert_eq!(KeySelector::parse("row-0").unwrap(), KeySelector::Row(0));
        assert_eq!(KeySelector::parse("row-3").unwrap(), KeySelector::Row(3));
    }

    #[test]
    fn test_parse_column() {
        assert_eq!(KeySelector::parse("col-0").unwrap(), KeySelector::Column(0));
        assert_eq!(KeySelector::parse("col-7").unwrap(), KeySelector::Column(7));
    }

    #[test]
    fn test_parse_default() {
        assert_eq!(KeySelector::parse("default").unwrap(), KeySelector::Default);
    }

    #[test]
    fn test_parse_invalid() {
        assert!(KeySelector::parse("").is_err());
        assert!(KeySelector::parse("foo").is_err());
        assert!(KeySelector::parse("row-").is_err());
        assert!(KeySelector::parse("col-").is_err());
        assert!(KeySelector::parse("-5").is_err());
        assert!(KeySelector::parse("row-abc").is_err());
    }

    #[test]
    fn test_resolve_single() {
        let device = xl_device();
        assert_eq!(KeySelector::Single(0).resolve(&device).unwrap(), vec![0]);
        assert_eq!(KeySelector::Single(31).resolve(&device).unwrap(), vec![31]);
    }

    #[test]
    fn test_resolve_single_out_of_range() {
        let device = xl_device();
        assert!(KeySelector::Single(32).resolve(&device).is_err());
        assert!(KeySelector::Single(255).resolve(&device).is_err());
    }

    #[test]
    fn test_resolve_range() {
        let device = xl_device();
        assert_eq!(
            KeySelector::Range { start: 8, end: 15 }
                .resolve(&device)
                .unwrap(),
            vec![8, 9, 10, 11, 12, 13, 14, 15]
        );
        assert_eq!(
            KeySelector::Range { start: 0, end: 0 }
                .resolve(&device)
                .unwrap(),
            vec![0]
        );
    }

    #[test]
    fn test_resolve_range_out_of_range() {
        let device = xl_device();
        assert!(
            KeySelector::Range { start: 0, end: 32 }
                .resolve(&device)
                .is_err()
        );
    }

    #[test]
    fn test_resolve_row_xl() {
        let device = xl_device();
        // XL is 8x4, row 0 is keys 0-7
        assert_eq!(
            KeySelector::Row(0).resolve(&device).unwrap(),
            vec![0, 1, 2, 3, 4, 5, 6, 7]
        );
        // Row 3 is keys 24-31
        assert_eq!(
            KeySelector::Row(3).resolve(&device).unwrap(),
            vec![24, 25, 26, 27, 28, 29, 30, 31]
        );
    }

    #[test]
    fn test_resolve_row_mini() {
        let device = mini_device();
        // Mini is 3x2, row 0 is keys 0-2
        assert_eq!(KeySelector::Row(0).resolve(&device).unwrap(), vec![0, 1, 2]);
        assert_eq!(KeySelector::Row(1).resolve(&device).unwrap(), vec![3, 4, 5]);
    }

    #[test]
    fn test_resolve_column_xl() {
        let device = xl_device();
        // XL is 8x4, column 0 is keys 0, 8, 16, 24
        assert_eq!(
            KeySelector::Column(0).resolve(&device).unwrap(),
            vec![0, 8, 16, 24]
        );
        // Column 7 is keys 7, 15, 23, 31
        assert_eq!(
            KeySelector::Column(7).resolve(&device).unwrap(),
            vec![7, 15, 23, 31]
        );
    }

    #[test]
    fn test_resolve_column_mini() {
        let device = mini_device();
        // Mini is 3x2, column 0 is keys 0, 3
        assert_eq!(KeySelector::Column(0).resolve(&device).unwrap(), vec![0, 3]);
        assert_eq!(KeySelector::Column(2).resolve(&device).unwrap(), vec![2, 5]);
    }

    #[test]
    fn test_resolve_row_out_of_range() {
        let device = xl_device();
        assert!(KeySelector::Row(4).resolve(&device).is_err()); // XL has rows 0-3
    }

    #[test]
    fn test_resolve_column_out_of_range() {
        let device = xl_device();
        assert!(KeySelector::Column(8).resolve(&device).is_err()); // XL has cols 0-7
    }

    #[test]
    fn test_resolve_default() {
        let device = xl_device();
        let empty: Vec<u8> = vec![];
        assert_eq!(KeySelector::Default.resolve(&device).unwrap(), empty);
    }

    #[test]
    fn test_priority_ordering() {
        assert!(
            KeySelector::Single(0).priority() < KeySelector::Range { start: 0, end: 5 }.priority()
        );
        assert!(
            KeySelector::Range { start: 0, end: 5 }.priority() < KeySelector::Row(0).priority()
        );
        assert!(KeySelector::Row(0).priority() == KeySelector::Column(0).priority());
        assert!(KeySelector::Row(0).priority() < KeySelector::Default.priority());
    }

    #[test]
    fn test_might_match() {
        assert!(KeySelector::Single(5).might_match(5));
        assert!(!KeySelector::Single(5).might_match(6));

        assert!(KeySelector::Range { start: 5, end: 10 }.might_match(7));
        assert!(!KeySelector::Range { start: 5, end: 10 }.might_match(11));

        // Row/Column/Default always return true (need device for exact check)
        assert!(KeySelector::Row(0).might_match(100));
        assert!(KeySelector::Column(0).might_match(100));
        assert!(KeySelector::Default.might_match(100));
    }

    #[test]
    fn test_display() {
        assert_eq!(KeySelector::Single(5).to_string(), "5");
        assert_eq!(KeySelector::Range { start: 8, end: 15 }.to_string(), "8-15");
        assert_eq!(KeySelector::Row(2).to_string(), "row-2");
        assert_eq!(KeySelector::Column(3).to_string(), "col-3");
        assert_eq!(KeySelector::Default.to_string(), "default");
    }

    #[test]
    fn test_from_str() {
        let selector: KeySelector = "row-2".parse().unwrap();
        assert_eq!(selector, KeySelector::Row(2));
    }

    #[test]
    fn test_serde_roundtrip() {
        let selectors = vec![
            KeySelector::Single(5),
            KeySelector::Range { start: 8, end: 15 },
            KeySelector::Row(2),
            KeySelector::Column(3),
            KeySelector::Default,
        ];

        for selector in selectors {
            let json = serde_json::to_string(&selector).unwrap();
            let parsed: KeySelector = serde_json::from_str(&json).unwrap();
            assert_eq!(selector, parsed);
        }
    }

    #[test]
    fn test_yaml_deserialize() {
        let yaml = r#""row-0""#;
        let selector: KeySelector = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(selector, KeySelector::Row(0));

        let yaml = r#""8-15""#;
        let selector: KeySelector = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(selector, KeySelector::Range { start: 8, end: 15 });
    }

    #[test]
    fn test_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(KeySelector::Single(5));
        set.insert(KeySelector::Row(0));
        set.insert(KeySelector::Single(5)); // Duplicate

        assert_eq!(set.len(), 2);
    }
}
