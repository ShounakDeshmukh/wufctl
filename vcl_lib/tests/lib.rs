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
        Err(VclError::ApiError { code, message }) => {
            assert_eq!(code, 3);
            assert!(message.contains("Access denied"));
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

// ===== Input validation tests =====
//
// Every VclClient method below validates its arguments before making any
// network call, so these can run against a dummy endpoint without a live
// VCL server: an invalid argument must produce InvalidParameter, not a
// network error, proving validation runs first.

fn invalid_param_client() -> VclClient {
    VclClient::new("http://127.0.0.1:0/unused", "test_token")
}

#[tokio::test]
async fn test_test_rejects_empty_message() {
    let result = invalid_param_client().test("").await;
    assert!(matches!(result, Err(VclError::InvalidParameter(_))));
}

#[tokio::test]
async fn test_add_request_rejects_non_positive_image_id() {
    let result = invalid_param_client()
        .add_request(0, "now", 60, None, false)
        .await;
    assert!(matches!(result, Err(VclError::InvalidParameter(_))));
}

#[tokio::test]
async fn test_add_request_rejects_empty_start() {
    let result = invalid_param_client()
        .add_request(1, "", 60, None, false)
        .await;
    assert!(matches!(result, Err(VclError::InvalidParameter(_))));
}

#[tokio::test]
async fn test_add_request_rejects_non_positive_length() {
    let result = invalid_param_client()
        .add_request(1, "now", 0, None, false)
        .await;
    assert!(matches!(result, Err(VclError::InvalidParameter(_))));
}

#[tokio::test]
async fn test_add_request_with_ending_rejects_non_positive_end() {
    let result = invalid_param_client()
        .add_request_with_ending(1, "now", 0, None, false)
        .await;
    assert!(matches!(result, Err(VclError::InvalidParameter(_))));
}

#[tokio::test]
async fn test_deploy_server_rejects_non_positive_image_id() {
    let result = invalid_param_client()
        .deploy_server(0, "now", 3600, DeployServerOptions::default())
        .await;
    assert!(matches!(result, Err(VclError::InvalidParameter(_))));
}

#[tokio::test]
async fn test_get_request_status_rejects_non_positive_id() {
    let result = invalid_param_client().get_request_status(0).await;
    assert!(matches!(result, Err(VclError::InvalidParameter(_))));
}

#[tokio::test]
async fn test_get_request_connect_data_rejects_empty_ip() {
    let result = invalid_param_client().get_request_connect_data(1, "").await;
    assert!(matches!(result, Err(VclError::InvalidParameter(_))));
}

#[tokio::test]
async fn test_extend_request_rejects_non_positive_extend_time() {
    let result = invalid_param_client().extend_request(1, 0).await;
    assert!(matches!(result, Err(VclError::InvalidParameter(_))));
}

#[tokio::test]
async fn test_set_request_ending_rejects_non_positive_end() {
    let result = invalid_param_client().set_request_ending(1, 0).await;
    assert!(matches!(result, Err(VclError::InvalidParameter(_))));
}

#[tokio::test]
async fn test_end_request_rejects_non_positive_id() {
    let result = invalid_param_client().end_request(0).await;
    assert!(matches!(result, Err(VclError::InvalidParameter(_))));
}

#[tokio::test]
async fn test_auto_capture_rejects_non_positive_id() {
    let result = invalid_param_client().auto_capture(0).await;
    assert!(matches!(result, Err(VclError::InvalidParameter(_))));
}

#[tokio::test]
async fn test_get_group_images_rejects_empty_name() {
    let result = invalid_param_client().get_group_images("").await;
    assert!(matches!(result, Err(VclError::InvalidParameter(_))));
}

#[tokio::test]
async fn test_add_image_to_group_rejects_non_positive_image_id() {
    let result = invalid_param_client().add_image_to_group("group", 0).await;
    assert!(matches!(result, Err(VclError::InvalidParameter(_))));
}

#[tokio::test]
async fn test_remove_image_from_group_rejects_empty_name() {
    let result = invalid_param_client().remove_image_from_group("", 1).await;
    assert!(matches!(result, Err(VclError::InvalidParameter(_))));
}

#[tokio::test]
async fn test_add_image_group_to_computer_group_rejects_empty_computer_group() {
    let result = invalid_param_client()
        .add_image_group_to_computer_group("images", "")
        .await;
    assert!(matches!(result, Err(VclError::InvalidParameter(_))));
}

#[tokio::test]
async fn test_remove_image_group_from_computer_group_rejects_empty_image_group() {
    let result = invalid_param_client()
        .remove_image_group_from_computer_group("", "computers")
        .await;
    assert!(matches!(result, Err(VclError::InvalidParameter(_))));
}

#[tokio::test]
async fn test_node_exists_rejects_empty_node_name() {
    let result = invalid_param_client().node_exists("", "parent").await;
    assert!(matches!(result, Err(VclError::InvalidParameter(_))));
}

#[tokio::test]
async fn test_add_node_rejects_empty_parent_node() {
    let result = invalid_param_client().add_node("child", "").await;
    assert!(matches!(result, Err(VclError::InvalidParameter(_))));
}

#[tokio::test]
async fn test_remove_node_rejects_non_positive_id() {
    let result = invalid_param_client().remove_node(0).await;
    assert!(matches!(result, Err(VclError::InvalidParameter(_))));
}

#[tokio::test]
async fn test_get_user_group_privs_rejects_non_positive_node_id() {
    let result = invalid_param_client()
        .get_user_group_privs("group", "Local", 0)
        .await;
    assert!(matches!(result, Err(VclError::InvalidParameter(_))));
}

#[tokio::test]
async fn test_add_user_group_priv_rejects_empty_permissions() {
    let result = invalid_param_client()
        .add_user_group_priv("group", "Local", 1, "")
        .await;
    assert!(matches!(result, Err(VclError::InvalidParameter(_))));
}

#[tokio::test]
async fn test_remove_user_group_priv_rejects_empty_affiliation() {
    let result = invalid_param_client()
        .remove_user_group_priv("group", "", 1, "manageGroup")
        .await;
    assert!(matches!(result, Err(VclError::InvalidParameter(_))));
}

#[tokio::test]
async fn test_get_resource_group_privs_rejects_empty_resource_type() {
    let result = invalid_param_client()
        .get_resource_group_privs("group", "", 1)
        .await;
    assert!(matches!(result, Err(VclError::InvalidParameter(_))));
}

#[tokio::test]
async fn test_add_resource_group_priv_rejects_non_positive_node_id() {
    let result = invalid_param_client()
        .add_resource_group_priv("group", "image", 0, "manageGroup")
        .await;
    assert!(matches!(result, Err(VclError::InvalidParameter(_))));
}

#[tokio::test]
async fn test_remove_resource_group_priv_rejects_empty_name() {
    let result = invalid_param_client()
        .remove_resource_group_priv("", "image", 1, "manageGroup")
        .await;
    assert!(matches!(result, Err(VclError::InvalidParameter(_))));
}

#[tokio::test]
async fn test_get_user_group_attributes_rejects_empty_name() {
    let result = invalid_param_client()
        .get_user_group_attributes("", "Local")
        .await;
    assert!(matches!(result, Err(VclError::InvalidParameter(_))));
}

#[tokio::test]
async fn test_add_user_group_rejects_empty_owner() {
    let max_times = UserGroupMaxTimes {
        initial_max_time: 60,
        total_max_time: 480,
        max_extend_time: 60,
        custom: true,
    };
    let result = invalid_param_client()
        .add_user_group("group", "Local", "", "managers", max_times)
        .await;
    assert!(matches!(result, Err(VclError::InvalidParameter(_))));
}

#[tokio::test]
async fn test_edit_user_group_rejects_empty_new_name() {
    let result = invalid_param_client()
        .edit_user_group("group", "Local", "", "Local", UserGroupEdits::default())
        .await;
    assert!(matches!(result, Err(VclError::InvalidParameter(_))));
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
        Err(VclError::ApiError { code, message }) => {
            assert_eq!(code, 1);
            assert!(message.contains("Invalid parameters"));
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
        Err(VclError::ApiError { code, message }) => {
            assert_eq!(code, 2);
            assert!(message.contains("Reservation failed"));
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
fn test_parse_array_of_structs() {
    // Shape returned by XMLRPCgetImages, XMLRPCgetRequestIds, XMLRPCgetUserGroups, etc.
    let xml = r#"<?xml version="1.0"?>
<methodResponse>
  <params>
    <param>
      <value>
        <array>
          <data>
            <value>
              <struct>
                <member><name>id</name><value><int>1</int></value></member>
                <member><name>name</name><value><string>image-one</string></value></member>
              </struct>
            </value>
            <value>
              <struct>
                <member><name>id</name><value><int>2</int></value></member>
                <member><name>name</name><value><string>image-two</string></value></member>
              </struct>
            </value>
          </data>
        </array>
      </value>
    </param>
  </params>
</methodResponse>"#;

    let result = parse_response(xml).unwrap();
    assert!(result.is_array());
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 2);

    assert_eq!(arr[0].get("id").and_then(|v| v.as_i64()), Some(1));
    assert_eq!(
        arr[0].get("name").and_then(|v| v.as_str()),
        Some("image-one")
    );
    assert_eq!(arr[1].get("id").and_then(|v| v.as_i64()), Some(2));
    assert_eq!(
        arr[1].get("name").and_then(|v| v.as_str()),
        Some("image-two")
    );
}

#[test]
fn test_parse_struct_containing_struct() {
    let xml = r#"<?xml version="1.0"?>
<methodResponse>
  <params>
    <param>
      <value>
        <struct>
          <member>
            <name>outer</name>
            <value><string>outer-value</string></value>
          </member>
          <member>
            <name>nested</name>
            <value>
              <struct>
                <member><name>inner</name><value><string>inner-value</string></value></member>
              </struct>
            </value>
          </member>
        </struct>
      </value>
    </param>
  </params>
</methodResponse>"#;

    let result = parse_response(xml).unwrap();
    assert_eq!(
        result.get("outer").and_then(|v| v.as_str()),
        Some("outer-value")
    );
    let nested = result.get("nested").expect("should have 'nested' member");
    assert!(nested.is_object());
    assert_eq!(
        nested.get("inner").and_then(|v| v.as_str()),
        Some("inner-value")
    );
}

#[test]
fn test_parse_deeply_nested_mixed_structure() {
    // struct -> array -> struct -> array, to confirm depth tracking holds
    // through multiple levels and multiple sibling types.
    let xml = r#"<?xml version="1.0"?>
<methodResponse>
  <params>
    <param>
      <value>
        <struct>
          <member>
            <name>items</name>
            <value>
              <array>
                <data>
                  <value>
                    <struct>
                      <member>
                        <name>tags</name>
                        <value>
                          <array>
                            <data>
                              <value><string>a</string></value>
                              <value><string>b</string></value>
                            </data>
                          </array>
                        </value>
                      </member>
                      <member><name>id</name><value><int>7</int></value></member>
                    </struct>
                  </value>
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
    let items = result.get("items").unwrap().as_array().unwrap();
    assert_eq!(items.len(), 1);

    let item = &items[0];
    assert_eq!(item.get("id").and_then(|v| v.as_i64()), Some(7));

    let tags = item.get("tags").unwrap().as_array().unwrap();
    assert_eq!(tags.len(), 2);
    assert_eq!(tags[0].as_str(), Some("a"));
    assert_eq!(tags[1].as_str(), Some("b"));
}

#[test]
fn test_parse_response_invalid_int_is_error() {
    let xml = r#"<?xml version="1.0"?>
<methodResponse>
  <params>
    <param>
      <value><int>not-a-number</int></value>
    </param>
  </params>
</methodResponse>"#;

    let result = parse_response(xml);
    assert!(matches!(result, Err(VclError::XmlParseError(_))));
}

#[test]
fn test_parse_response_invalid_boolean_is_error() {
    let xml = r#"<?xml version="1.0"?>
<methodResponse>
  <params>
    <param>
      <value><boolean>maybe</boolean></value>
    </param>
  </params>
</methodResponse>"#;

    let result = parse_response(xml);
    assert!(matches!(result, Err(VclError::XmlParseError(_))));
}

#[test]
fn test_parse_response_missing_params_is_error() {
    let xml = r#"<?xml version="1.0"?>
<methodResponse>
</methodResponse>"#;

    let result = parse_response(xml);
    assert!(matches!(result, Err(VclError::InvalidResponse(_))));
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
