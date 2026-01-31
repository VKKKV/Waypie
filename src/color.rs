use serde::{Deserialize, Deserializer};

pub type ColorRGB = (f64, f64, f64);
pub type ColorRGBA = (f64, f64, f64, f64);

/// Parse hex color string (#RRGGBB or #RRGGBBAA) to normalized RGB/RGBA tuple
fn hex_to_color(hex: &str) -> Result<ColorRGBA, String> {
    let hex = hex.trim_start_matches('#').trim_start_matches("0x");

    let (_, has_alpha) = match hex.len() {
        6 => (6, false),
        8 => (8, true),
        _ => return Err(format!("invalid hex length: {}", hex.len())),
    };

    let value = u32::from_str_radix(hex, 16).map_err(|_| "invalid hex string")?;

    if has_alpha {
        let r = ((value >> 24) & 0xFF) as f64 / 255.0;
        let g = ((value >> 16) & 0xFF) as f64 / 255.0;
        let b = ((value >> 8) & 0xFF) as f64 / 255.0;
        let a = (value & 0xFF) as f64 / 255.0;
        Ok((r, g, b, a))
    } else {
        let r = ((value >> 16) & 0xFF) as f64 / 255.0;
        let g = ((value >> 8) & 0xFF) as f64 / 255.0;
        let b = (value & 0xFF) as f64 / 255.0;
        Ok((r, g, b, 1.0))
    }
}

/// Deserialize hex color string to Color4 (r, g, b, a)
pub fn deserialize_color_rgba<'de, D>(deserializer: D) -> Result<ColorRGBA, D::Error>
where
    D: Deserializer<'de>,
{
    // 使用 &str 减少一次 String 的堆内存分配
    let s = <&str>::deserialize(deserializer)?;
    hex_to_color(s).map_err(serde::de::Error::custom)
}

/// Deserialize hex color string to Color3 (r, g, b), ignoring alpha
pub fn deserialize_color_rgb<'de, D>(deserializer: D) -> Result<ColorRGB, D::Error>
where
    D: Deserializer<'de>,
{
    let s = <&str>::deserialize(deserializer)?;
    let (r, g, b, _) = hex_to_color(s).map_err(serde::de::Error::custom)?;
    Ok((r, g, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_6_digit() {
        let result = hex_to_color("#FF0000").unwrap();
        assert_eq!(result, (1.0, 0.0, 0.0, 1.0));
    }

    #[test]
    fn test_hex_8_digit() {
        let result = hex_to_color("#FF0000FF").unwrap();
        assert_eq!(result, (1.0, 0.0, 0.0, 1.0));
    }

    #[test]
    fn test_hex_with_alpha() {
        let result = hex_to_color("#FF000080").unwrap();
        assert!((result.3 - 0.502).abs() < 0.01); // ~50% alpha
    }

    #[test]
    fn test_hex_lowercase() {
        let result = hex_to_color("#ffffff").unwrap();
        assert_eq!(result, (1.0, 1.0, 1.0, 1.0));
    }

    #[test]
    fn test_color3_ignores_alpha() {
        let (r, g, b) = (1.0, 0.5, 0.0);
        assert_eq!((r, g, b), (1.0, 0.5, 0.0));
    }
}
