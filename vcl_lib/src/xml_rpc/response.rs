use crate::errors::{Result, VclError};
use crate::value::Value;
use std::collections::HashMap;

/// Parse XML-RPC response according to XML-RPC specification
/// Returns Value::Struct for method responses or fault information
pub fn parse_response(xml: &str) -> Result<Value> {
    // Check for fault
    if xml.contains("<fault>") {
        return parse_fault(xml);
    }

    // Extract methodResponse content
    let start = xml
        .find("<methodResponse>")
        .ok_or_else(|| VclError::InvalidResponse("No methodResponse found".into()))?;
    let end = xml
        .find("</methodResponse>")
        .ok_or_else(|| VclError::InvalidResponse("No methodResponse closing tag".into()))?;

    let content = &xml[start + 16..end];

    // Extract params
    let params_start = content
        .find("<params>")
        .ok_or_else(|| VclError::InvalidResponse("No params found".into()))?;
    let params_end = content
        .find("</params>")
        .ok_or_else(|| VclError::InvalidResponse("No params closing tag".into()))?;

    let params_content = &content[params_start + 8..params_end];

    // Extract param
    let param_start = params_content
        .find("<param>")
        .ok_or_else(|| VclError::InvalidResponse("No param found".into()))?;
    let param_end = params_content
        .find("</param>")
        .ok_or_else(|| VclError::InvalidResponse("No param closing tag".into()))?;

    let param_content = &params_content[param_start + 7..param_end];

    // Extract value
    parse_value(param_content)
}

fn parse_fault(xml: &str) -> Result<Value> {
    let fault_start = xml
        .find("<fault>")
        .ok_or_else(|| VclError::InvalidResponse("Malformed fault".into()))?;
    let fault_end = xml
        .find("</fault>")
        .ok_or_else(|| VclError::InvalidResponse("Malformed fault".into()))?;

    let fault_content = &xml[fault_start + 7..fault_end];
    let value = parse_value(fault_content)?;

    // Extract error code and message from struct
    if let Value::Struct(map) = &value {
        let code = map.get("faultCode").and_then(|v| v.as_i64()).unwrap_or(-1);
        let msg = map
            .get("faultString")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error");
        return Err(VclError::ApiError(format!("Fault [{}] {}", code, msg)));
    }

    Err(VclError::ApiError(format!("Fault: {}", value)))
}

fn parse_value(content: &str) -> Result<Value> {
    let content = content.trim();

    // Find <value> tags
    let start = content
        .find("<value>")
        .ok_or_else(|| VclError::XmlParseError("No value tag".into()))?;
    let end = content
        .rfind("</value>")
        .ok_or_else(|| VclError::XmlParseError("No closing value tag".into()))?;

    let inner = &content[start + 7..end].trim();

    parse_value_inner(inner)
}

fn parse_value_inner(content: &str) -> Result<Value> {
    let content = content.trim();

    // Check for struct FIRST (before scalars that might be nested inside)
    if content.contains("<struct>") {
        return parse_struct(content);
    }

    // Check for array
    if content.contains("<array>") {
        return parse_array(content);
    }

    // Now check for scalar types
    // Check for int (<i4> or <int> per XML-RPC spec)
    if let Some(start) = content.find("<i4>") {
        if let Some(end) = content.find("</i4>") {
            let num_str = content[start + 4..end].trim();
            return Ok(Value::Int(num_str.parse::<i64>().unwrap_or(0)));
        }
    }

    if let Some(start) = content.find("<int>") {
        if let Some(end) = content.find("</int>") {
            let num_str = content[start + 5..end].trim();
            return Ok(Value::Int(num_str.parse::<i64>().unwrap_or(0)));
        }
    }

    // Check for boolean (0 or 1 per XML-RPC spec)
    if let Some(start) = content.find("<boolean>") {
        if let Some(end) = content.find("</boolean>") {
            let bool_str = content[start + 9..end].trim();
            return Ok(Value::Bool(bool_str == "1"));
        }
    }

    // Check for string
    if let Some(start) = content.find("<string>") {
        if let Some(end) = content.find("</string>") {
            let s = &content[start + 8..end];
            return Ok(Value::String(unescape_xml(s)));
        }
    }

    // Check for double
    if let Some(start) = content.find("<double>") {
        if let Some(end) = content.find("</double>") {
            let num_str = content[start + 8..end].trim();
            return Ok(Value::Double(num_str.parse::<f64>().unwrap_or(0.0)));
        }
    }

    // Default to string if no type tag
    Ok(Value::String(unescape_xml(content)))
}

fn parse_array(content: &str) -> Result<Value> {
    let data_start = content
        .find("<data>")
        .ok_or_else(|| VclError::XmlParseError("No data tag in array".into()))?;
    let data_end = content
        .find("</data>")
        .ok_or_else(|| VclError::XmlParseError("No closing data tag".into()))?;

    let data_content = &content[data_start + 6..data_end];
    let mut items = Vec::new();

    // Find all <value> tags
    let mut pos = 0;
    while let Some(val_start) = data_content[pos..].find("<value>") {
        let abs_start = pos + val_start;
        if let Some(val_end) = data_content[abs_start..].find("</value>") {
            let abs_end = abs_start + val_end + 8;
            let val_xml = &data_content[abs_start..abs_end];
            if let Ok(v) = parse_value(val_xml) {
                items.push(v);
            }
            pos = abs_end;
        } else {
            break;
        }
    }

    Ok(Value::Array(items))
}

fn parse_struct(content: &str) -> Result<Value> {
    let struct_start = content
        .find("<struct>")
        .ok_or_else(|| VclError::XmlParseError("No struct tag".into()))?;
    let struct_end = content
        .find("</struct>")
        .ok_or_else(|| VclError::XmlParseError("No closing struct tag".into()))?;

    let struct_content = &content[struct_start + 8..struct_end];
    let mut members = HashMap::new();

    // Find all <member> tags
    let mut pos = 0;
    while let Some(mem_start) = struct_content[pos..].find("<member>") {
        let abs_start = pos + mem_start;
        if let Some(mem_end) = struct_content[abs_start..].find("</member>") {
            let abs_end = abs_start + mem_end + 9;
            let member_xml = &struct_content[abs_start + 8..abs_end - 9];

            // Parse name
            if let Some(name_start) = member_xml.find("<name>") {
                if let Some(name_end) = member_xml.find("</name>") {
                    let name = member_xml[name_start + 6..name_end].to_string();

                    // Parse value
                    if let Some(val_start) = member_xml.find("<value>") {
                        if let Some(val_end) = member_xml.rfind("</value>") {
                            let val_xml = &member_xml[val_start..val_end + 8];
                            if let Ok(v) = parse_value(val_xml) {
                                members.insert(name, v);
                            }
                        }
                    }
                }
            }

            pos = abs_end;
        } else {
            break;
        }
    }

    Ok(Value::Struct(members))
}

fn unescape_xml(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}
