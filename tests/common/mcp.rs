//! Raw JSON-RPC over `POST /api/mcp`, as an MCP client sends it.

use axum::Router;
use axum::body::Body;
use axum::http::{HeaderMap, Method, Request, StatusCode};
use serde_json::{Value, json};

use super::{TEST_TOKEN, send};

/// A POST to `/api/mcp` with the headers a Streamable HTTP client sends.
pub fn request(body: &Value, bearer: Option<&str>) -> Request<Body> {
    let mut b = Request::builder()
        .method(Method::POST)
        .uri("/api/mcp")
        .header("host", super::TEST_HOST)
        .header("content-type", "application/json")
        .header("accept", "application/json, text/event-stream");
    if let Some(t) = bearer {
        b = b.header("authorization", format!("Bearer {t}"));
    }
    b.body(Body::from(body.to_string())).expect("build request")
}

/// Send `body` with the test token; status, headers and the JSON answer
/// (`Null` when empty).
pub async fn post(app: &Router, body: &Value) -> (StatusCode, HeaderMap, Value) {
    let resp = send(app.clone(), request(body, Some(TEST_TOKEN))).await;
    let status = resp.status();
    let headers = resp.headers().clone();
    let bytes = axum::body::to_bytes(resp.into_body(), 1 << 22)
        .await
        .expect("read body");
    let json = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes)
            .unwrap_or_else(|e| panic!("JSON-RPC body ({e}): {}", String::from_utf8_lossy(&bytes)))
    };
    (status, headers, json)
}

/// One JSON-RPC request; returns the whole response object (200 asserted).
pub async fn rpc(app: &Router, method: &str, params: Value) -> Value {
    let body = json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params });
    let (status, _, resp) = post(app, &body).await;
    assert_eq!(status, StatusCode::OK, "{resp}");
    assert_eq!(resp["jsonrpc"], "2.0");
    assert_eq!(resp["id"], 1);
    resp
}

/// `tools/call`; returns the `result` (a JSON-RPC error fails the test).
pub async fn call_tool(app: &Router, name: &str, args: Value) -> Value {
    let resp = rpc(
        app,
        "tools/call",
        json!({ "name": name, "arguments": args }),
    )
    .await;
    assert!(resp.get("error").is_none(), "{name}: {resp}");
    let result = resp["result"].clone();
    // The text content is the same JSON as `structuredContent`.
    let text = result["content"][0]["text"].as_str().expect("text content");
    let parsed: Value = serde_json::from_str(text).expect("text is JSON");
    assert_eq!(parsed, result["structuredContent"], "{name}");
    result
}

/// A successful tool call's `structuredContent`.
pub async fn tool(app: &Router, name: &str, args: Value) -> Value {
    let result = call_tool(app, name, args).await;
    assert_eq!(result["isError"], false, "{name}: {result}");
    result["structuredContent"].clone()
}

/// A failing tool call's REST error body.
pub async fn tool_error(app: &Router, name: &str, args: Value) -> Value {
    let result = call_tool(app, name, args).await;
    assert_eq!(result["isError"], true, "{name}: {result}");
    result["structuredContent"].clone()
}

/// `tools/call` with `token` on `host`; the `result` object.
pub async fn tool_as(app: &Router, host: &str, token: &str, name: &str, args: Value) -> Value {
    let body = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call",
                       "params": { "name": name, "arguments": args } });
    let mut req = request(&body, Some(token));
    req.headers_mut().insert(
        "host",
        axum::http::HeaderValue::from_str(host).expect("host header"),
    );
    let resp = send(app.clone(), req).await;
    assert_eq!(resp.status(), StatusCode::OK, "{name}");
    let bytes = axum::body::to_bytes(resp.into_body(), 1 << 22)
        .await
        .expect("read body");
    let v: Value = serde_json::from_slice(&bytes).expect("JSON-RPC body");
    assert!(v.get("error").is_none(), "{name}: {v}");
    v["result"].clone()
}
