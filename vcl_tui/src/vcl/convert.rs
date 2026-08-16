use color_eyre::eyre::{ContextCompat, Report, Result, eyre};
use vcl_lib::Value;

use super::model::{
    ActionResult, ConnectData, ConnectDataResult, ConnectMethod, Image, RequestListEntry,
    RequestStatus,
};

/// Coerces a `Value` into an `i64`, accepting either a JSON number or a string that parses as an integer
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

fn field_str_array(v: &Value, key: &str) -> Result<Vec<String>> {
    field(v, key)?
        .as_array()
        .with_context(|| format!("field `{key}` is not an array"))?
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_owned)
                .with_context(|| format!("field `{key}` contains a non-string element"))
        })
        .collect()
}

impl TryFrom<&Value> for Image {
    type Error = Report;

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

impl TryFrom<&Value> for RequestStatus {
    type Error = Report;

    fn try_from(v: &Value) -> Result<Self> {
        Ok(RequestStatus {
            status: field_str(v, "status")?,
            time: field(v, "time")
                .ok()
                .map(|t| coerce_i64(t, "time"))
                .transpose()?,
        })
    }
}

impl TryFrom<&Value> for ActionResult {
    type Error = Report;

    fn try_from(v: &Value) -> Result<Self> {
        if field_str(v, "status")? == "success" {
            Ok(ActionResult::Success {
                requestid: field(v, "requestid")
                    .ok()
                    .map(|r| coerce_i64(r, "requestid"))
                    .transpose()?,
            })
        } else {
            Ok(ActionResult::Error {
                errorcode: coerce_i64(field(v, "errorcode")?, "errorcode")?,
                errormsg: field_str(v, "errormsg")?,
            })
        }
    }
}

impl TryFrom<&Value> for RequestListEntry {
    type Error = Report;

    fn try_from(v: &Value) -> Result<Self> {
        Ok(RequestListEntry {
            requestid: coerce_i64(field(v, "requestid")?, "requestid")?,
            imagename: field_str(v, "imagename")?,
            start: coerce_i64(field(v, "start")?, "start")?,
            end: coerce_i64(field(v, "end")?, "end")?,
        })
    }
}

fn connect_method(id: &str, v: &Value) -> Result<ConnectMethod> {
    Ok(ConnectMethod {
        id: id.to_string(),
        description: field_str(v, "description")?,
        connectports: field_str_array(v, "connectports")?,
    })
}

impl TryFrom<&Value> for ConnectDataResult {
    type Error = Report;

    fn try_from(v: &Value) -> Result<Self> {
        if field_str(v, "status")? == "notready" {
            return Ok(ConnectDataResult::NotReady);
        }
        let Value::Struct(methods) = field(v, "connectMethods")? else {
            return Err(eyre!("expected `connectMethods` to be a struct"));
        };
        let mut connect_methods = methods
            .iter()
            .map(|(id, mv)| connect_method(id, mv))
            .collect::<Result<Vec<_>>>()?;
        connect_methods.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(ConnectDataResult::Ready(ConnectData {
            server_ip: field_str(v, "serverIP")?,
            user: field_str(v, "user")?,
            password: field_str(v, "password")?,
            connect_port: field_str(v, "connectport")?,
            connect_methods,
        }))
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

    #[test]
    fn request_status_loading_has_time() {
        let mut map = HashMap::new();
        map.insert("status".to_string(), Value::String("loading".to_string()));
        map.insert("time".to_string(), Value::Int(1));
        let status = RequestStatus::try_from(&Value::Struct(map)).unwrap();
        assert_eq!(status.status, "loading");
        assert_eq!(status.time, Some(1));
    }

    #[test]
    fn request_status_ready_has_no_time() {
        let mut map = HashMap::new();
        map.insert("status".to_string(), Value::String("ready".to_string()));
        let status = RequestStatus::try_from(&Value::Struct(map)).unwrap();
        assert_eq!(status.status, "ready");
        assert_eq!(status.time, None);
    }

    #[test]
    fn action_result_success() {
        let mut map = HashMap::new();
        map.insert("status".to_string(), Value::String("success".to_string()));
        map.insert(
            "requestid".to_string(),
            Value::String("4214683".to_string()),
        );
        match ActionResult::try_from(&Value::Struct(map)).unwrap() {
            ActionResult::Success { requestid } => assert_eq!(requestid, Some(4214683)),
            ActionResult::Error { .. } => panic!("expected Success"),
        }
    }

    #[test]
    fn action_result_success_without_requestid() {
        let mut map = HashMap::new();
        map.insert("status".to_string(), Value::String("success".to_string()));
        match ActionResult::try_from(&Value::Struct(map)).unwrap() {
            ActionResult::Success { requestid } => assert_eq!(requestid, None),
            ActionResult::Error { .. } => panic!("expected Success"),
        }
    }

    #[test]
    fn action_result_error() {
        let mut map = HashMap::new();
        map.insert("status".to_string(), Value::String("error".to_string()));
        map.insert("errorcode".to_string(), Value::Int(24));
        map.insert(
            "errormsg".to_string(),
            Value::String("reservation length exceeds max".to_string()),
        );
        match ActionResult::try_from(&Value::Struct(map)).unwrap() {
            ActionResult::Success { .. } => panic!("expected Error"),
            ActionResult::Error {
                errorcode,
                errormsg,
            } => {
                assert_eq!(errorcode, 24);
                assert_eq!(errormsg, "reservation length exceeds max");
            }
        }
    }

    #[test]
    fn request_list_entry_from_live_shape() {
        // Fixture captured live - each `requests[]` element is a full struct, not a bare scalar.
        let mut map = HashMap::new();
        map.insert("isserver".to_string(), Value::Int(0));
        map.insert("serverowner".to_string(), Value::Int(1));
        map.insert("state".to_string(), Value::String("reserved".to_string()));
        map.insert("imageid".to_string(), Value::String("7414".to_string()));
        map.insert("ostype".to_string(), Value::String("linux".to_string()));
        map.insert(
            "requestid".to_string(),
            Value::String("4214792".to_string()),
        );
        map.insert(
            "imagename".to_string(),
            Value::String("Ubuntu 22 GPU with Cuda (GeForce RTX 2080 Ti)".to_string()),
        );
        map.insert(
            "OS".to_string(),
            Value::String("Ubuntu (VMware)".to_string()),
        );
        map.insert("end".to_string(), Value::Int(1786684500));
        map.insert("admin".to_string(), Value::Int(1));
        map.insert("start".to_string(), Value::Int(1786662000));

        let entry = RequestListEntry::try_from(&Value::Struct(map)).unwrap();
        assert_eq!(entry.requestid, 4214792);
        assert_eq!(
            entry.imagename,
            "Ubuntu 22 GPU with Cuda (GeForce RTX 2080 Ti)"
        );
        assert_eq!(entry.start, 1786662000);
        assert_eq!(entry.end, 1786684500);
    }

    #[test]
    fn connect_data_not_ready() {
        let mut map = HashMap::new();
        map.insert("status".to_string(), Value::String("notready".to_string()));
        match ConnectDataResult::try_from(&Value::Struct(map)).unwrap() {
            ConnectDataResult::NotReady => {}
            ConnectDataResult::Ready(_) => panic!("expected NotReady"),
        }
    }

    #[test]
    fn connect_data_ready_from_live_shape() {
        let mut ssh = HashMap::new();
        ssh.insert(
            "description".to_string(),
            Value::String("SSH (Secure Shell) on Port 22".to_string()),
        );
        ssh.insert("connecttext".to_string(), Value::String(String::new()));
        ssh.insert(
            "connectports".to_string(),
            Value::Array(vec![Value::String("TCP:22:22".to_string())]),
        );

        let mut methods = HashMap::new();
        methods.insert("1".to_string(), Value::Struct(ssh));

        let mut map = HashMap::new();
        map.insert("status".to_string(), Value::String("ready".to_string()));
        map.insert(
            "serverIP".to_string(),
            Value::String("152.7.179.113".to_string()),
        );
        map.insert("user".to_string(), Value::String("sdeshmu4".to_string()));
        map.insert(
            "password".to_string(),
            Value::String("(use your campus password)".to_string()),
        );
        map.insert("connectport".to_string(), Value::String("22".to_string()));
        map.insert("connectMethods".to_string(), Value::Struct(methods));

        match ConnectDataResult::try_from(&Value::Struct(map)).unwrap() {
            ConnectDataResult::Ready(data) => {
                assert_eq!(data.server_ip, "152.7.179.113");
                assert_eq!(data.user, "sdeshmu4");
                assert_eq!(data.connect_port, "22");
                assert_eq!(data.connect_methods.len(), 1);
                assert_eq!(data.connect_methods[0].id, "1");
                assert_eq!(data.connect_methods[0].connectports, vec!["TCP:22:22"]);
            }
            ConnectDataResult::NotReady => panic!("expected Ready"),
        }
    }
}
