use serde::{Deserialize, Deserializer};

pub type ColorRGB = (f64, f64, f64);
pub type ColorRGBA = (f64, f64, f64, f64);

// 1. 定义一个特征，用于从 RGBA 数据构造目标类型
// 这允许我们将构造逻辑（是丢弃 Alpha 还是保留 Alpha）委托给具体类型去处理
pub trait FromColorValues {
    fn from_rgba(r: f64, g: f64, b: f64, a: f64) -> Self;
}

// 2. 为 ColorRGB (3元组) 实现特征：忽略 Alpha
impl FromColorValues for ColorRGB {
    fn from_rgba(r: f64, g: f64, b: f64, _a: f64) -> Self {
        (r, g, b)
    }
}

// 2. 为 ColorRGBA (4元组) 实现特征：保留 Alpha
impl FromColorValues for ColorRGBA {
    fn from_rgba(r: f64, g: f64, b: f64, a: f64) -> Self {
        (r, g, b, a)
    }
}

// 3. 将解析逻辑改为泛型 T，T 必须实现 FromColorValues
fn hex_to_color<T: FromColorValues>(hex: &str) -> Result<T, String> {
    let hex = hex.trim_start_matches('#').trim_start_matches("0x");

    let (_, has_alpha) = match hex.len() {
        6 => (6, false),
        8 => (8, true),
        _ => return Err(format!("invalid hex length: {}", hex.len())),
    };

    let value = u32::from_str_radix(hex, 16).map_err(|_| "invalid hex string")?;

    // 计算分量
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
            1.0, // 默认 Alpha 为 1.0
        )
    };

    // 使用特征方法构造目标类型 T
    Ok(T::from_rgba(r, g, b, a))
}

// 4. 通用的反序列化函数
// 这里的 T 会由 Serde 根据结构体字段的类型自动推断
pub fn deserialize_color<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: FromColorValues, // 约束 T 必须是我们支持的颜色类型
{
    // 依然使用 &str 借用来优化性能
    let s = <&str>::deserialize(deserializer)?;
    hex_to_color(s).map_err(serde::de::Error::custom)
}
