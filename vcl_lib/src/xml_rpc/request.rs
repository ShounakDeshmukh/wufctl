use crate::value::Value;

/// Build XML-RPC request from method name and arguments
pub fn build_request(method: &str, args: Vec<Value>) -> String {
    let mut xml = String::from("<?xml version=\"1.0\"?>\n");
    xml.push_str("<methodCall>\n");
    xml.push_str(&format!(
        "  <methodName>{}</methodName>\n",
        escape_xml(method)
    ));

    // Always include params, even if empty (required by XML-RPC spec)
    xml.push_str("  <params>\n");
    for arg in args {
        xml.push_str("    <param>\n");
        xml.push_str(&format!("      {}\n", value_to_xml(&arg, 3)));
        xml.push_str("    </param>\n");
    }
    xml.push_str("  </params>\n");

    xml.push_str("</methodCall>");
    xml
}

fn value_to_xml(value: &Value, indent: usize) -> String {
    let ind = " ".repeat(indent);

    match value {
        // Null is represented as empty string in XML-RPC
        Value::Null => format!("{}<value><string></string></value>", ind),

        // Boolean: must be 0 or 1 per XML-RPC spec
        Value::Bool(b) => format!(
            "{}<value><boolean>{}</boolean></value>",
            ind,
            if *b { 1 } else { 0 }
        ),

        // Integers: use <i4> tag (also supports <int>)
        Value::Int(i) => format!("{}<value><i4>{}</i4></value>", ind, i),

        // Floats: use <double> tag
        Value::Double(f) => format!("{}<value><double>{}</double></value>", ind, f),

        // Strings
        Value::String(s) => format!("{}<value><string>{}</string></value>", ind, escape_xml(s)),

        // Arrays: per XML-RPC spec, wrapped in <array><data>
        Value::Array(arr) => {
            let mut result = format!(
                "{}<value>\n{}<array>\n{}<data>\n",
                ind,
                " ".repeat(indent + 2),
                " ".repeat(indent + 2)
            );
            for item in arr {
                result.push_str(&format!(
                    "{}{}\n",
                    " ".repeat(indent + 4),
                    value_to_xml(item, indent + 4)
                ));
            }
            result.push_str(&format!(
                "{}</data>\n{}</array>\n{}</value>",
                " ".repeat(indent + 2),
                " ".repeat(indent + 2),
                ind
            ));
            result
        }

        // Structs: per XML-RPC spec, contains <member> elements with <name> and <value>
        Value::Struct(map) => {
            let mut result = format!("{}<value>\n{}<struct>\n", ind, " ".repeat(indent + 2));
            for (key, val) in map {
                result.push_str(&format!("{}<member>\n", " ".repeat(indent + 2)));
                result.push_str(&format!(
                    "{}<name>{}</name>\n",
                    " ".repeat(indent + 4),
                    escape_xml(key)
                ));
                result.push_str(&format!(
                    "{}{}\n",
                    " ".repeat(indent + 4),
                    value_to_xml(val, indent + 4)
                ));
                result.push_str(&format!("{}</member>\n", " ".repeat(indent + 2)));
            }
            result.push_str(&format!(
                "{}</struct>\n{}</value>",
                " ".repeat(indent + 2),
                ind
            ));
            result
        }
    }
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_request() {
        let req = build_request("XMLRPCtest", vec![Value::String("hello".to_string())]);
        assert!(req.contains("<methodName>XMLRPCtest</methodName>"));
        assert!(req.contains("hello"));
    }
}
