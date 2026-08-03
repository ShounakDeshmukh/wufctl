use std::collections::HashMap;
use vcl_lib::*;

#[test]
fn test_build_request_string() {
    let request = build_request("XMLRPCtest", vec![Value::String("hello".to_string())]);
    assert!(request.contains("XMLRPCtest"));
    assert!(request.contains("hello"));
    assert!(request.contains("<string>"));
}

#[test]
fn test_build_request_int() {
    let request = build_request("test", vec![Value::Int(42)]);
    assert!(request.contains("<i4>42</i4>") || request.contains("<int>42</int>"));
}

#[test]
fn test_build_request_bool() {
    let request = build_request("test", vec![Value::Bool(true)]);
    assert!(request.contains("<boolean>1</boolean>"));

    let request = build_request("test", vec![Value::Bool(false)]);
    assert!(request.contains("<boolean>0</boolean>"));
}

#[test]
fn test_build_request_array() {
    let request = build_request(
        "test",
        vec![Value::Array(vec![
            Value::String("a".to_string()),
            Value::String("b".to_string()),
            Value::String("c".to_string()),
        ])],
    );
    assert!(request.contains("<array>"));
    assert!(request.contains("<data>"));
}

#[test]
fn test_build_request_struct() {
    let mut map = HashMap::new();
    map.insert("key".to_string(), Value::String("value".to_string()));
    let request = build_request("test", vec![Value::Struct(map)]);
    assert!(request.contains("<struct>"));
    assert!(request.contains("<member>"));
    assert!(request.contains("<name>key</name>"));
}

#[test]
fn test_parse_response_string() {
    let xml = r#"<?xml version="1.0"?>
<methodResponse>
  <params>
    <param>
      <value><string>hello</string></value>
    </param>
  </params>
</methodResponse>"#;

    let result = parse_response(xml).unwrap();
    assert_eq!(result.as_str(), Some("hello"));
}

#[test]
fn test_parse_response_int() {
    let xml = r#"<?xml version="1.0"?>
<methodResponse>
  <params>
    <param>
      <value><int>42</int></value>
    </param>
  </params>
</methodResponse>"#;

    let result = parse_response(xml).unwrap();
    assert_eq!(result.as_i64(), Some(42));
}

#[test]
fn test_parse_response_bool() {
    let xml = r#"<?xml version="1.0"?>
<methodResponse>
  <params>
    <param>
      <value><boolean>1</boolean></value>
    </param>
  </params>
</methodResponse>"#;

    let result = parse_response(xml).unwrap();
    assert_eq!(result.as_bool(), Some(true));
}

#[test]
fn test_parse_response_fault() {
    let xml = r#"<?xml version="1.0"?>
<methodResponse>
<fault>
 <value>
  <struct>
   <member>
    <name>faultString</name>
    <value>
     <string>Access denied</string>
    </value>
   </member>
   <member>
    <name>faultCode</name>
    <value>
     <int>3</int>
    </value>
   </member>
  </struct>
 </value>
</fault>
</methodResponse>"#;

    let result = parse_response(xml);
    assert!(result.is_err());
    match result {
        Err(VclError::ApiError(msg)) => {
            assert!(msg.contains("Fault [3]"));
            assert!(msg.contains("Access denied"));
        }
        _ => panic!("Expected ApiError"),
    }
}

#[test]
fn test_parse_response_array() {
    let xml = r#"<?xml version="1.0"?>
<methodResponse>
  <params>
    <param>
      <value>
        <array>
          <data>
            <value><string>a</string></value>
            <value><string>b</string></value>
            <value><string>c</string></value>
          </data>
        </array>
      </value>
    </param>
  </params>
</methodResponse>"#;

    let result = parse_response(xml).unwrap();
    assert!(result.is_array());
    assert_eq!(result.as_array().unwrap().len(), 3);
}

#[test]
fn test_parse_response_struct() {
    let xml = r#"<?xml version="1.0"?>
<methodResponse>
  <params>
    <param>
      <value>
        <struct>
          <member>
            <name>name</name>
            <value><string>John</string></value>
          </member>
          <member>
            <name>age</name>
            <value><int>30</int></value>
          </member>
        </struct>
      </value>
    </param>
  </params>
</methodResponse>"#;

    let result = parse_response(xml).unwrap();
    assert!(result.is_object());
    assert_eq!(result.get("name").and_then(|v| v.as_str()), Some("John"));
    assert_eq!(result.get("age").and_then(|v| v.as_i64()), Some(30));
}

#[test]
fn test_vcl_client_creation() {
    let client = VclClient::new(
        "https://vcl.ncsu.edu/scheduling/index.php?mode=xmlrpccall",
        "test_token_123",
    );
    // Just verify it constructs without panic
    assert!(!client.endpoint().is_empty());
}

#[test]
fn test_xmlrpc_test_request_format() {
    let request = build_request(
        "XMLRPCtest",
        vec![Value::String("hello from Rust".to_string())],
    );
    assert!(request.contains("<methodName>XMLRPCtest</methodName>"));
    assert!(request.contains("hello from Rust"));
    assert!(request.contains("<?xml version"));
    assert!(request.contains("<methodCall>"));
    assert!(request.contains("</methodCall>"));
}

#[test]
fn test_parse_xmlrpc_test_response() {
    let xml = r#"<?xml version="1.0"?>
<methodResponse>
  <params>
    <param>
      <value><string>hello from Rust</string></value>
    </param>
  </params>
</methodResponse>"#;

    let result = parse_response(xml).unwrap();
    assert_eq!(result.as_str(), Some("hello from Rust"));
}

#[test]
fn test_fault_code_1_invalidparam() {
    let xml = r#"<?xml version="1.0"?>
<methodResponse>
<fault>
 <value>
  <struct>
   <member>
    <name>faultString</name>
    <value><string>Invalid parameters</string></value>
   </member>
   <member>
    <name>faultCode</name>
    <value><int>1</int></value>
   </member>
  </struct>
 </value>
</fault>
</methodResponse>"#;

    let result = parse_response(xml);
    assert!(result.is_err());
    match result {
        Err(VclError::ApiError(msg)) => {
            assert!(msg.contains("Fault [1]"));
            assert!(msg.contains("Invalid parameters"));
        }
        _ => panic!("Expected ApiError"),
    }
}

#[test]
fn test_fault_code_2_reservation_error() {
    let xml = r#"<?xml version="1.0"?>
<methodResponse>
<fault>
 <value>
  <struct>
   <member>
    <name>faultString</name>
    <value><string>Reservation failed</string></value>
   </member>
   <member>
    <name>faultCode</name>
    <value><int>2</int></value>
   </member>
  </struct>
 </value>
</fault>
</methodResponse>"#;

    let result = parse_response(xml);
    assert!(result.is_err());
    match result {
        Err(VclError::ApiError(msg)) => {
            assert!(msg.contains("Fault [2]"));
            assert!(msg.contains("Reservation failed"));
        }
        _ => panic!("Expected ApiError"),
    }
}

#[test]
fn test_struct_with_array_member() {
    let xml = r#"<?xml version="1.0"?>
<methodResponse>
  <params>
    <param>
      <value>
        <struct>
          <member>
            <name>ids</name>
            <value>
              <array>
                <data>
                  <value><int>1001</int></value>
                  <value><int>1002</int></value>
                  <value><int>1003</int></value>
                </data>
              </array>
            </value>
          </member>
        </struct>
      </value>
    </param>
  </params>
</methodResponse>"#;

    let result = parse_response(xml).unwrap();
    assert!(result.is_object());

    if let Some(ids) = result.get("ids") {
        assert!(ids.is_array());
        let arr = ids.as_array().unwrap();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0].as_i64(), Some(1001));
        assert_eq!(arr[1].as_i64(), Some(1002));
        assert_eq!(arr[2].as_i64(), Some(1003));
    } else {
        panic!("Should have 'ids' member");
    }
}

#[test]
fn test_response_with_xml_entities() {
    let xml = r#"<?xml version="1.0"?>
<methodResponse>
  <params>
    <param>
      <value><string>Test &lt;tag&gt; with &amp; ampersand</string></value>
    </param>
  </params>
</methodResponse>"#;

    let result = parse_response(xml).unwrap();
    assert_eq!(result.as_str(), Some("Test <tag> with & ampersand"));
}

#[test]
fn test_empty_array_response() {
    let xml = r#"<?xml version="1.0"?>
<methodResponse>
  <params>
    <param>
      <value>
        <array>
          <data>
          </data>
        </array>
      </value>
    </param>
  </params>
</methodResponse>"#;

    let result = parse_response(xml).unwrap();
    assert!(result.is_array());
    assert_eq!(result.as_array().unwrap().len(), 0);
}

#[test]
fn test_multiple_params_uses_first() {
    let xml = r#"<?xml version="1.0"?>
<methodResponse>
  <params>
    <param>
      <value><string>first</string></value>
    </param>
    <param>
      <value><string>second</string></value>
    </param>
  </params>
</methodResponse>"#;

    let result = parse_response(xml).unwrap();
    // Should only parse the first param
    assert_eq!(result.as_str(), Some("first"));
}

#[test]
fn test_value_len() {
    let arr = Value::Array(vec![Value::Int(1), Value::Int(2)]);
    assert_eq!(arr.len(), Some(2));

    let s = Value::String("hello".to_string());
    assert_eq!(s.len(), Some(5));

    let i = Value::Int(42);
    assert_eq!(i.len(), None);
}

#[test]
fn test_value_is_empty() {
    let arr_empty = Value::Array(vec![]);
    assert!(arr_empty.is_empty());

    let arr_full = Value::Array(vec![Value::Int(1)]);
    assert!(!arr_full.is_empty());

    let s_empty = Value::String("".to_string());
    assert!(s_empty.is_empty());

    let s_full = Value::String("hello".to_string());
    assert!(!s_full.is_empty());
}

#[test]
fn test_value_to_json_string() {
    assert_eq!(Value::Null.to_json_string(), "null");
    assert_eq!(Value::Bool(true).to_json_string(), "true");
    assert_eq!(Value::Int(42).to_json_string(), "42");
    assert_eq!(
        Value::String("hello".to_string()).to_json_string(),
        "\"hello\""
    );
}

#[test]
fn test_value_json_string_escaping() {
    let s = Value::String("hello\"world".to_string());
    assert!(s.to_json_string().contains("\\\""));

    let s = Value::String("line1\nline2".to_string());
    assert!(s.to_json_string().contains("\\n"));
}

#[test]
fn test_parse_nested_array() {
    let xml = r#"<?xml version="1.0"?>
<methodResponse>
  <params>
    <param>
      <value>
        <array>
          <data>
            <value><int>1</int></value>
            <value><int>2</int></value>
            <value><int>3</int></value>
          </data>
        </array>
      </value>
    </param>
  </params>
</methodResponse>"#;

    let result = parse_response(xml).unwrap();
    assert!(result.as_array().is_some());
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0].as_i64(), Some(1));
    assert_eq!(arr[1].as_i64(), Some(2));
    assert_eq!(arr[2].as_i64(), Some(3));
}

#[test]
fn test_parse_struct_with_multiple_types() {
    let xml = r#"<?xml version="1.0"?>
<methodResponse>
  <params>
    <param>
      <value>
        <struct>
          <member>
            <name>count</name>
            <value><int>5</int></value>
          </member>
          <member>
            <name>enabled</name>
            <value><boolean>1</boolean></value>
          </member>
          <member>
            <name>name</name>
            <value><string>test</string></value>
          </member>
        </struct>
      </value>
    </param>
  </params>
</methodResponse>"#;

    let result = parse_response(xml).unwrap();
    assert_eq!(result.get("count").and_then(|v| v.as_i64()), Some(5));
    assert_eq!(result.get("enabled").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(result.get("name").and_then(|v| v.as_str()), Some("test"));
}
