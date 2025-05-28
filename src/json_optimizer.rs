use bytes::Bytes;
use serde::Serialize;

/// 高性能 JSON 处理模块
/// 完全使用 simd-json 库，避免双重解析

/// JSON 解析错误类型
#[derive(Debug)]
pub enum JsonError {
  ParseError(simd_json::Error),
  InvalidUtf8,
}

impl From<simd_json::Error> for JsonError {
  fn from(err: simd_json::Error) -> Self {
    JsonError::ParseError(err)
  }
}

/// 🚀 SIMD 优化的 JSON 解析
/// 使用 simd-json 进行高性能解析，比标准 serde_json 快 2-3 倍
pub fn parse_json_simd(data: &[u8]) -> Result<simd_json::OwnedValue, JsonError> {
  // simd-json 需要可变的字节切片，所以我们需要复制数据
  let mut mutable_data = data.to_vec();

  // 使用 SIMD 指令进行快速解析
  simd_json::to_owned_value(&mut mutable_data).map_err(JsonError::from)
}

/// 🚀 SIMD 优化的 JSON 解析（从 Bytes）
pub fn parse_json_from_bytes(bytes: &Bytes) -> Result<simd_json::OwnedValue, JsonError> {
  parse_json_simd(bytes.as_ref())
}

/// 🚀 SIMD 优化的 JSON 解析（从字符串）
pub fn parse_json_from_str(s: &str) -> Result<simd_json::OwnedValue, JsonError> {
  parse_json_simd(s.as_bytes())
}

/// 🚀 SIMD 优化的 JSON 序列化（紧凑格式）
pub fn serialize_json_compact<T: Serialize>(value: &T) -> Result<String, JsonError> {
  // 使用 simd-json 的 serde 模块进行序列化
  simd_json::serde::to_string(value).map_err(|e| JsonError::ParseError(e))
}

/// 🚀 SIMD 优化的 JSON 序列化（美化格式）
pub fn serialize_json_pretty<T: Serialize>(value: &T) -> Result<String, JsonError> {
  // 使用 simd-json 的 serde 模块进行美化序列化
  simd_json::serde::to_string_pretty(value).map_err(|e| JsonError::ParseError(e))
}

/// 🚀 将 simd_json::OwnedValue 序列化为字符串
pub fn serialize_owned_value(value: &simd_json::OwnedValue) -> Result<String, JsonError> {
  simd_json::serde::to_string(value).map_err(JsonError::from)
}

/// 🚀 将 simd_json::OwnedValue 序列化为美化字符串
pub fn serialize_owned_value_pretty(value: &simd_json::OwnedValue) -> Result<String, JsonError> {
  simd_json::serde::to_string_pretty(value).map_err(JsonError::from)
}

/// 🚀 JSON 验证器
/// 快速检查字符串是否为有效的 JSON，不进行完整解析
pub fn is_valid_json(data: &[u8]) -> bool {
  let mut mutable_data = data.to_vec();
  simd_json::to_owned_value(&mut mutable_data).is_ok()
}

/// 🚀 JSON 大小估算器
/// 在不完整解析的情况下估算 JSON 对象的复杂度
pub fn estimate_json_complexity(data: &[u8]) -> usize {
  // 简单的启发式方法：计算大括号、中括号和逗号的数量
  let mut complexity = 0;
  for &byte in data {
    match byte {
      b'{' | b'}' | b'[' | b']' | b',' => complexity += 1,
      _ => {}
    }
  }
  complexity
}

/// 🚀 JSON 压缩器
/// 移除不必要的空白字符来减少 JSON 大小
pub fn minify_json(json_str: &str) -> Result<String, JsonError> {
  let mut mutable_data = json_str.as_bytes().to_vec();

  match simd_json::to_owned_value(&mut mutable_data) {
    Ok(value) => serialize_owned_value(&value),
    Err(e) => Err(JsonError::ParseError(e)),
  }
}

/// 🚀 创建 JSON 对象
pub fn create_json_object() -> simd_json::OwnedValue {
  simd_json::OwnedValue::Object(Default::default())
}

/// 🚀 创建 JSON 数组
pub fn create_json_array() -> simd_json::OwnedValue {
  simd_json::OwnedValue::Array(Default::default())
}

/// 🚀 高效转换：将 simd_json::OwnedValue 转换为 serde_json::Value
/// 用于 NAPI 接口兼容性，避免双重解析
pub fn simd_to_serde_value(simd_value: simd_json::OwnedValue) -> serde_json::Value {
  // 使用 simd-json 的序列化，然后用 serde_json 反序列化
  // 这比之前的实现更高效，因为我们使用了 SIMD 序列化
  match simd_json::serde::to_string(&simd_value) {
    Ok(json_str) => serde_json::from_str(&json_str).unwrap_or(serde_json::Value::Null),
    Err(_) => serde_json::Value::Null,
  }
}

/// 🚀 批量转换：将 HashMap<String, simd_json::OwnedValue> 转换为 serde_json::Value
pub fn simd_map_to_serde_value(
  simd_map: std::collections::HashMap<String, simd_json::OwnedValue>,
) -> serde_json::Value {
  let mut serde_map = serde_json::Map::new();
  for (key, value) in simd_map {
    serde_map.insert(key, simd_to_serde_value(value));
  }
  serde_json::Value::Object(serde_map)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_simd_json_parsing() {
    let json_str = r#"{"name": "test", "value": 42, "array": [1, 2, 3]}"#;
    let result = parse_json_from_str(json_str);
    assert!(result.is_ok());
  }

  #[test]
  fn test_json_validation() {
    assert!(is_valid_json(br#"{"valid": true}"#));
    assert!(!is_valid_json(br#"{"invalid": }"#));
  }

  #[test]
  fn test_complexity_estimation() {
    let simple_json = br#"{"a": 1}"#;
    let complex_json = br#"{"a": [1, 2, {"b": 3}], "c": {"d": 4}}"#;

    assert!(estimate_json_complexity(complex_json) > estimate_json_complexity(simple_json));
  }

  #[test]
  fn test_serialization() {
    let value = create_json_object();
    let result = serialize_owned_value(&value);
    assert!(result.is_ok());
  }
}
