use crate::errors::{Result, VclError};
use crate::value::Value;
use roxmltree::Node;
use std::collections::HashMap;

/// Parse an XML-RPC response according to the XML-RPC specification.
///
/// # Errors
///
/// Returns `VclError::ApiError` if the response is a `<fault>`.
/// Returns `VclError::InvalidResponse` or `VclError::XmlParseError` if the
/// response is malformed or contains an unsupported value type.
pub fn parse_response(xml: &str) -> Result<Value> {
    let doc =
        roxmltree::Document::parse(xml).map_err(|e| VclError::XmlParseError(e.to_string()))?;

    let response = doc.root_element();
    if response.tag_name().name() != "methodResponse" {
        return Err(VclError::InvalidResponse(
            "Root element is not <methodResponse>".into(),
        ));
    }

    if let Some(fault) = child_element(response, "fault") {
        return Err(parse_fault(fault));
    }

    let params = child_element(response, "params")
        .ok_or_else(|| VclError::InvalidResponse("No <params> found".into()))?;
    let param = child_element(params, "param")
        .ok_or_else(|| VclError::InvalidResponse("No <param> found".into()))?;
    let value_node = child_element(param, "value")
        .ok_or_else(|| VclError::InvalidResponse("No <value> found in <param>".into()))?;

    value_from_node(value_node)
}

fn parse_fault(fault: Node) -> VclError {
    let result = child_element(fault, "value")
        .ok_or_else(|| VclError::InvalidResponse("Malformed fault: no <value>".into()))
        .and_then(value_from_node);

    match result {
        Ok(Value::Struct(map)) => {
            let code = map.get("faultCode").and_then(Value::as_i64).unwrap_or(-1);
            let message = map
                .get("faultString")
                .and_then(Value::as_str)
                .unwrap_or("Unknown error")
                .to_string();
            VclError::ApiError { code, message }
        }
        Ok(other) => VclError::ApiError {
            code: -1,
            message: other.to_string(),
        },
        Err(e) => e,
    }
}

/// Walks the parsed DOM tree rather than scanning raw text, so nesting depth is never miscounted.
fn value_from_node(value_node: Node) -> Result<Value> {
    // XML-RPC permits a bare, untyped string: <value>hello</value>
    let Some(type_node) = value_node.children().find(|n| n.is_element()) else {
        return Ok(Value::String(text_content(value_node)));
    };

    match type_node.tag_name().name() {
        "i4" | "int" => text_content(type_node)
            .trim()
            .parse::<i64>()
            .map(Value::Int)
            .map_err(|e| VclError::XmlParseError(format!("Invalid integer: {}", e))),
        "double" => text_content(type_node)
            .trim()
            .parse::<f64>()
            .map(Value::Double)
            .map_err(|e| VclError::XmlParseError(format!("Invalid double: {}", e))),
        "boolean" => match text_content(type_node).trim() {
            "1" => Ok(Value::Bool(true)),
            "0" => Ok(Value::Bool(false)),
            other => Err(VclError::XmlParseError(format!(
                "Invalid boolean value: {}",
                other
            ))),
        },
        "string" => Ok(Value::String(text_content(type_node))),
        "nil" => Ok(Value::Null),
        "array" => array_from_node(type_node),
        "struct" => struct_from_node(type_node),
        other => Err(VclError::XmlParseError(format!(
            "Unsupported XML-RPC type: <{}>",
            other
        ))),
    }
}

fn array_from_node(array_node: Node) -> Result<Value> {
    let data = child_element(array_node, "data")
        .ok_or_else(|| VclError::XmlParseError("Array missing <data>".into()))?;

    data.children()
        .filter(|n| n.is_element() && n.has_tag_name("value"))
        .map(value_from_node)
        .collect::<Result<Vec<_>>>()
        .map(Value::Array)
}

fn struct_from_node(struct_node: Node) -> Result<Value> {
    let mut members = HashMap::new();

    for member in struct_node
        .children()
        .filter(|n| n.is_element() && n.has_tag_name("member"))
    {
        let name = child_element(member, "name")
            .map(text_content)
            .ok_or_else(|| VclError::XmlParseError("Struct member missing <name>".into()))?;
        let value_node = child_element(member, "value")
            .ok_or_else(|| VclError::XmlParseError("Struct member missing <value>".into()))?;
        members.insert(name, value_from_node(value_node)?);
    }

    Ok(Value::Struct(members))
}

fn child_element<'a, 'input>(node: Node<'a, 'input>, tag: &str) -> Option<Node<'a, 'input>> {
    node.children()
        .find(|n| n.is_element() && n.has_tag_name(tag))
}

/// `roxmltree` already decodes XML entities while parsing, so no manual unescaping is needed here.
fn text_content(node: Node) -> String {
    node.children().filter_map(|n| n.text()).collect()
}
