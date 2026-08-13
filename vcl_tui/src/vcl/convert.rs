use color_eyre::eyre::{ContextCompat, Result};
use vcl_lib::Value;

use super::model::Image;

/// Accepts `Value::Int` or a parseable `Value::String` - the server's
/// inconsistent numeric typing (see RESPONSE_SHAPES.md).
pub fn coerce_i64(v: &Value, field: &'static str) -> Result<i64> {
    v.as_i64()
        .or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
        .with_context(|| format!("field `{field}` has an unexpected type"))
}

fn field<'a>(v: &'a Value, key: &str) -> Result<&'a Value> {
    v.get(key).with_context(|| format!("missing field `{key}`"))
}

fn field_str(v: &Value, key: &str) -> Result<String> {
    field(v, key)?
        .as_str()
        .map(str::to_owned)
        .with_context(|| format!("field `{key}` is not a string"))
}

impl TryFrom<&Value> for Image {
    type Error = color_eyre::eyre::Report;

    fn try_from(v: &Value) -> Result<Self> {
        Ok(Image {
            id: coerce_i64(field(v, "id")?, "id")?,
            name: field_str(v, "name")?,
            ostype: field_str(v, "ostype")?,
            usage: field_str(v, "usage")?,
            description: field_str(v, "description")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn image_value(id: Value) -> Value {
        let mut map = HashMap::new();
        map.insert("id".to_string(), id);
        map.insert(
            "name".to_string(),
            Value::String("Ubuntu 22.04".to_string()),
        );
        map.insert("ostype".to_string(), Value::String("linux".to_string()));
        map.insert("usage".to_string(), Value::String(String::new()));
        map.insert(
            "description".to_string(),
            Value::String("A Linux image".to_string()),
        );
        Value::Struct(map)
    }

    #[test]
    fn parses_int_id() {
        let v = image_value(Value::Int(42));
        let img = Image::try_from(&v).unwrap();
        assert_eq!(img.id, 42);
        assert_eq!(img.name, "Ubuntu 22.04");
        assert!(img.is_reservable());
    }

    #[test]
    fn parses_string_id() {
        let v = image_value(Value::String("42".to_string()));
        let img = Image::try_from(&v).unwrap();
        assert_eq!(img.id, 42);
    }

    #[test]
    fn missing_field_errors() {
        let mut map = HashMap::new();
        map.insert("id".to_string(), Value::Int(1));
        let v = Value::Struct(map);
        assert!(Image::try_from(&v).is_err());
    }

    #[test]
    fn windows_image_is_not_reservable() {
        let mut map = HashMap::new();
        map.insert("id".to_string(), Value::Int(1));
        map.insert(
            "name".to_string(),
            Value::String("MATLAB (AVD)".to_string()),
        );
        map.insert("ostype".to_string(), Value::String("windows".to_string()));
        map.insert("usage".to_string(), Value::String(String::new()));
        map.insert("description".to_string(), Value::String(String::new()));
        let img = Image::try_from(&Value::Struct(map)).unwrap();
        assert!(!img.is_reservable());
    }
}
