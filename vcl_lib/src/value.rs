use std::collections::HashMap;

/// XML-RPC value type covering every tag the VCL API uses.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// Maps to XML-RPC `<nil>`.
    Null,
    /// Maps to XML-RPC `<boolean>`.
    Bool(bool),
    /// Maps to XML-RPC `<i4>` or `<int>`.
    Int(i64),
    /// Maps to XML-RPC `<double>`.
    Double(f64),
    /// Maps to XML-RPC `<string>`.
    String(String),
    /// Maps to XML-RPC `<array>`.
    Array(Vec<Value>),
    /// Maps to XML-RPC `<struct>`.
    Struct(HashMap<String, Value>),
}

impl Value {
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Double(f) => Some(*f),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(arr) => Some(arr),
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Struct(map) => map.get(key),
            _ => None,
        }
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut Value> {
        match self {
            Value::Struct(map) => map.get_mut(key),
            _ => None,
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    pub fn is_array(&self) -> bool {
        matches!(self, Value::Array(_))
    }

    pub fn is_object(&self) -> bool {
        matches!(self, Value::Struct(_))
    }

    pub fn len(&self) -> Option<usize> {
        match self {
            Value::Array(arr) => Some(arr.len()),
            Value::Struct(map) => Some(map.len()),
            Value::String(s) => Some(s.len()),
            _ => None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == Some(0)
    }

    pub fn to_display_string(&self) -> String {
        match self {
            Value::Null => "null".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Int(i) => i.to_string(),
            Value::Double(f) => f.to_string(),
            Value::String(s) => s.clone(),
            Value::Array(arr) => {
                let items = arr
                    .iter()
                    .map(|v| v.to_display_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{}]", items)
            }
            Value::Struct(map) => {
                let items = map
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v.to_display_string()))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{{}}}", items)
            }
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_display_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_creation() {
        let v = Value::Int(42);
        assert_eq!(v.as_i64(), Some(42));

        let v = Value::String("hello".to_string());
        assert_eq!(v.as_str(), Some("hello"));

        let v = Value::Bool(true);
        assert_eq!(v.as_bool(), Some(true));
    }

    #[test]
    fn test_value_array() {
        let arr = Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
        assert_eq!(arr.as_array().map(|a| a.len()), Some(3));
    }

    #[test]
    fn test_value_struct() {
        let mut map = HashMap::new();
        map.insert("key".to_string(), Value::String("value".to_string()));
        let v = Value::Struct(map);
        assert_eq!(v.get("key").and_then(|v| v.as_str()), Some("value"));
    }
}
