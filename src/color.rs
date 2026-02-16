use serde::{Deserialize, Deserializer};

/// Represents an RGB color as a tuple of three f64 values (0.0 to 1.0).
pub type ColorRGB = (f64, f64, f64);
/// Represents an RGBA color as a tuple of four f64 values (0.0 to 1.0).
pub type ColorRGBA = (f64, f64, f64, f64);

/// Trait for types that can be constructed from RGBA components.
pub trait FromColorValues {
    fn from_rgba(r: f64, g: f64, b: f64, a: f64) -> Self;
}

impl FromColorValues for ColorRGB {
    fn from_rgba(r: f64, g: f64, b: f64, _a: f64) -> Self {
        (r, g, b)
    }
}

impl FromColorValues for ColorRGBA {
    fn from_rgba(r: f64, g: f64, b: f64, a: f64) -> Self {
        (r, g, b, a)
    }
}

/// Parses a hex color string (e.g., "#RRGGBB" or "#RRGGBBAA") into a type T that implements FromColorValues.
pub fn hex_to_color<T: FromColorValues>(hex: &str) -> Result<T, String> {
    let hex = hex.trim_start_matches('#').trim_start_matches("0x");

    let (_, has_alpha) = match hex.len() {
        6 => (6, false),
        8 => (8, true),
        _ => return Err(format!("invalid hex length: {}", hex.len())),
    };

    let value = u32::from_str_radix(hex, 16).map_err(|_| "invalid hex string")?;

    let (r, g, b, a) = if has_alpha {
        (
            ((value >> 24) & 0xFF) as f64 / 255.0,
            ((value >> 16) & 0xFF) as f64 / 255.0,
            ((value >> 8) & 0xFF) as f64 / 255.0,
            (value & 0xFF) as f64 / 255.0,
        )
    } else {
        (
            ((value >> 16) & 0xFF) as f64 / 255.0,
            ((value >> 8) & 0xFF) as f64 / 255.0,
            (value & 0xFF) as f64 / 255.0,
            1.0,
        )
    };

    Ok(T::from_rgba(r, g, b, a))
}

/// Serde deserializer for hex color strings.
pub fn deserialize_color<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: FromColorValues,
{
    let s = <&str>::deserialize(deserializer)?;
    hex_to_color(s).map_err(serde::de::Error::custom)
}
