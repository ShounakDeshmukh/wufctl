use std::collections::HashMap;

/// XML-RPC value type following the XML-RPC specification
/// Supports all standard XML-RPC types used by the NCSU VCL API
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// Null/nil value
    Null,
    /// Boolean (maps to XML-RPC `<boolean>`)
    Bool(bool),
    /// Integer (maps to XML-RPC `<i4>` or `<int>`)
    Int(i64),
    /// Double precision floating point
    Double(f64),
    /// String data
    String(String),
    /// Array of values (maps to XML-RPC `<array>`)
    Array(Vec<Value>),
    /// Structure/object (maps to XML-RPC `<struct>`)
    Struct(HashMap<String, Value>),
}

impl Value {
    /// Get as boolean
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Get as integer
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// Get as float
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Double(f) => Some(*f),
            _ => None,
        }
    }

    /// Get as string
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// Get as array
    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(arr) => Some(arr),
            _ => None,
        }
    }

    /// Get struct field by key
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Struct(map) => map.get(key),
            _ => None,
        }
    }

    /// Get struct field by key (mutable)
    pub fn get_mut(&mut self, key: &str) -> Option<&mut Value> {
        match self {
            Value::Struct(map) => map.get_mut(key),
            _ => None,
        }
    }

    /// Check if value is null
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Check if value is an array
    pub fn is_array(&self) -> bool {
        matches!(self, Value::Array(_))
    }

    /// Check if value is a struct/object
    pub fn is_object(&self) -> bool {
        matches!(self, Value::Struct(_))
    }

    /// Convert to JSON-compatible string representation
    pub fn to_json_string(&self) -> String {
        match self {
            Value::Null => "null".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Int(i) => i.to_string(),
            Value::Double(f) => f.to_string(),
            Value::String(s) => format!("\"{}\"", escape_json_string(s)),
            Value::Array(arr) => {
                let items = arr.iter().map(|v| v.to_json_string()).collect::<Vec<_>>();
                format!("[{}]", items.join(","))
            }
            Value::Struct(map) => {
                let items = map
                    .iter()
                    .map(|(k, v)| format!("\"{}\":{}", escape_json_string(k), v.to_json_string()))
                    .collect::<Vec<_>>();
                format!("{{{}}}", items.join(","))
            }
        }
    }

    /// Get the length of an array or struct
    pub fn len(&self) -> Option<usize> {
        match self {
            Value::Array(arr) => Some(arr.len()),
            Value::Struct(map) => Some(map.len()),
            Value::String(s) => Some(s.len()),
            _ => None,
        }
    }

    /// Check if empty (for arrays, structs, or strings)
    pub fn is_empty(&self) -> bool {
        self.len() == Some(0)
    }

    /// Convert to string representation for Display
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

/// Escape a string for JSON output
fn escape_json_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
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
