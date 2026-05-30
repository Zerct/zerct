use serde_json::Value;

use crate::cli::project::string_field;

pub(super) fn print_list_result(response: &Value) {
    let empty_objects: &[Value] = &[];
    let objects = response
        .get("objects")
        .and_then(Value::as_array)
        .map_or(empty_objects, Vec::as_slice);
    if objects.is_empty() {
        println!("no storage objects");
        return;
    }

    for object in objects {
        let path = string_field(object, "path");
        let size = object
            .get("sizeBytes")
            .and_then(Value::as_u64)
            .map_or_else(String::new, |value| value.to_string());
        let status = string_field(object, "status");
        let public_url = object
            .get("publicUrl")
            .and_then(Value::as_str)
            .unwrap_or("");
        if public_url.is_empty() {
            println!("{path}\t{size}\t{status}");
        } else {
            println!("{path}\t{size}\t{status}\t{public_url}");
        }
    }
}

pub(super) fn print_upload_result(object: &Value) {
    let path = string_field(object, "path");
    let size = object
        .get("sizeBytes")
        .and_then(Value::as_u64)
        .map_or_else(String::new, |value| value.to_string());
    let public_url = object
        .get("publicUrl")
        .and_then(Value::as_str)
        .unwrap_or("");
    if public_url.is_empty() {
        println!("uploaded {path} {size}");
    } else {
        println!("uploaded {path} {size} {public_url}");
    }
}

pub(super) fn print_delete_result(response: &Value) {
    println!(
        "{} {}",
        if response
            .get("deleted")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            "deleted"
        } else {
            "not-found"
        },
        string_field(response, "path")
    );
}
