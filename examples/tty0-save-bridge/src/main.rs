use serde::Deserialize;
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
};
use tiny_http::{Header, Method, Response, Server, StatusCode};

const ADDRESS: &str = "127.0.0.1:4777";

#[derive(Deserialize)]
struct SaveVisualizerRequest {
    record_id: String,
    visualizer: Value,
}

fn main() {
    let server = Server::http(ADDRESS).expect("tty0 save bridge should bind");
    println!("tty0 save bridge listening on http://{ADDRESS}");

    for mut request in server.incoming_requests() {
        let method = request.method().clone();
        let url = request.url().to_string();
        let response = match (method, url.as_str()) {
            (Method::Options, "/save_visualizer") => cors_response(StatusCode(204), ""),
            (Method::Post, "/save_visualizer") => handle_save(&mut request),
            _ => cors_response(StatusCode(404), "not found"),
        };
        let _ = request.respond(response);
    }
}

fn handle_save(request: &mut tiny_http::Request) -> Response<std::io::Cursor<Vec<u8>>> {
    let mut body = String::new();
    if request.as_reader().read_to_string(&mut body).is_err() {
        return cors_response(StatusCode(400), "failed to read request body");
    }

    let payload: SaveVisualizerRequest = match serde_json::from_str(body.as_str()) {
        Ok(payload) => payload,
        Err(error) => {
            return cors_response(
                StatusCode(400),
                format!("invalid json payload: {error}").as_str(),
            )
        }
    };

    match write_visualizer(payload) {
        Ok(path) => cors_response(
            StatusCode(200),
            format!("saved {}", path.display()).as_str(),
        ),
        Err(error) => cors_response(StatusCode(500), error.as_str()),
    }
}

fn write_visualizer(payload: SaveVisualizerRequest) -> Result<PathBuf, String> {
    validate_record_id(payload.record_id.as_str())?;
    let record_path = records_dir().join(format!("{}.json", payload.record_id));
    let record_json = fs::read_to_string(&record_path)
        .map_err(|error| format!("failed to read {}: {error}", record_path.display()))?;
    let mut record_value: Value =
        serde_json::from_str(record_json.as_str()).map_err(|error| error.to_string())?;

    let media_page = record_value
        .get_mut("media_page")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "record missing media_page object".to_string())?;
    media_page.insert("visualizer".to_string(), payload.visualizer);

    let updated =
        serde_json::to_string_pretty(&record_value).map_err(|error| error.to_string())? + "\n";
    fs::write(&record_path, updated)
        .map_err(|error| format!("failed to write {}: {error}", record_path.display()))?;
    Ok(record_path)
}

fn records_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tty0/data/records")
        .canonicalize()
        .expect("tty0 records directory should exist")
}

fn validate_record_id(record_id: &str) -> Result<(), String> {
    if record_id.is_empty() {
        return Err("record_id must not be empty".to_string());
    }
    if record_id
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
    {
        Ok(())
    } else {
        Err("record_id contains unsupported characters".to_string())
    }
}

fn cors_response(status: StatusCode, body: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    let mut response = Response::from_string(body.to_string()).with_status_code(status);
    response.add_header(
        Header::from_bytes("Access-Control-Allow-Origin", "*").expect("valid allow-origin header"),
    );
    response.add_header(
        Header::from_bytes("Access-Control-Allow-Methods", "POST, OPTIONS")
            .expect("valid allow-methods header"),
    );
    response.add_header(
        Header::from_bytes("Access-Control-Allow-Headers", "*")
            .expect("valid allow-headers header"),
    );
    response
}
