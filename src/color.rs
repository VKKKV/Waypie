use serde::{Deserialize, Deserializer};

pub type Color3 = (f64, f64, f64);
pub type Color4 = (f64, f64, f64, f64);

/// Parse hex color string (#RRGGBB or #RRGGBBAA) to normalized RGB/RGBA tuple
fn hex_to_color(hex: &str) -> Result<Color4, String> {
    let hex = hex.trim_start_matches('#');

    match hex.len() {
        6 => {
            // #RRGGBB -> RGB with alpha 1.0
            let r = u8::from_str_radix(&hex[0..2], 16)
                .map_err(|_| "Invalid hex color")?;
            let g = u8::from_str_radix(&hex[2..4], 16)
                .map_err(|_| "Invalid hex color")?;
            let b = u8::from_str_radix(&hex[4..6], 16)
                .map_err(|_| "Invalid hex color")?;
            Ok((r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0, 1.0))
        }
        8 => {
            // #RRGGBBAA -> RGBA
            let r = u8::from_str_radix(&hex[0..2], 16)
                .map_err(|_| "Invalid hex color")?;
            let g = u8::from_str_radix(&hex[2..4], 16)
                .map_err(|_| "Invalid hex color")?;
            let b = u8::from_str_radix(&hex[4..6], 16)
                .map_err(|_| "Invalid hex color")?;
            let a = u8::from_str_radix(&hex[6..8], 16)
                .map_err(|_| "Invalid hex color")?;
            Ok((
                r as f64 / 255.0,
                g as f64 / 255.0,
                b as f64 / 255.0,
                a as f64 / 255.0,
            ))
        }
        _ => Err(format!("Invalid hex color length: {}", hex.len())),
    }
}

/// Deserialize hex color string to Color4 (r, g, b, a)
pub fn deserialize_color4<'de, D>(deserializer: D) -> Result<Color4, D::Error>
where
    D: Deserializer<'de>,
{
    let hex = String::deserialize(deserializer)?;
    hex_to_color(&hex).map_err(serde::de::Error::custom)
}

/// Deserialize hex color string to Color3 (r, g, b), ignoring alpha
pub fn deserialize_color3<'de, D>(deserializer: D) -> Result<Color3, D::Error>
where
    D: Deserializer<'de>,
{
    let hex = String::deserialize(deserializer)?;
    let (r, g, b, _) = hex_to_color(&hex).map_err(serde::de::Error::custom)?;
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
