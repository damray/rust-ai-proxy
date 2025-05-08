use axum::{
    body::{to_bytes, Body},
    extract::Request,
    http::StatusCode,
    response::Response,
    routing::{post, any},
    Router,
};
use futures_util::stream::TryStreamExt;
use reqwest::Client;
use serde_json::{json, Value};
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;

mod airs;

use airs::scan_with_airs;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/api/chat", post(handle_unified_prompt))
        .fallback(any(forward_to_ollama))
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("\u{1f680} Listening on http://{}", addr);

    axum::serve(tokio::net::TcpListener::bind(addr).await.unwrap(), app.into_make_service())
        .await
        .unwrap();
}

async fn handle_unified_prompt(mut req: Request) -> Result<Response, (StatusCode, String)> {
    let body = std::mem::take(req.body_mut());
    let body_bytes = to_bytes(body, 1024 * 1024)
        .await
        .map_err(|_| (StatusCode::BAD_REQUEST, "Failed to read request body".to_string()))?;

    let original_body: Value = serde_json::from_slice(&body_bytes)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid JSON format".to_string()))?;

    let prompt = original_body["messages"]
        .as_array()
        .and_then(|msgs| msgs.iter().rev().find(|m| m["role"] == "user"))
        .and_then(|m| m["content"].as_str())
        .unwrap_or("")
        .to_string();

    println!("\u{1f9e0} Prompt intercept\u{e9} : {}", prompt);

    match scan_with_airs(prompt.clone(), "".to_string()).await {
        Ok(scan_result) if scan_result.action == "allow" => {
            println!("\u{2705} Prompt autoris\u{e9} par AIRS");

            let client = Client::new();
            let ollama_res = client
                .post("http://ollama:11434/api/chat")
                .header("Content-Type", "application/json")
                .body(body_bytes.clone())
                .send()
                .await
                .map_err(|e| {
                    println!("\u{274c} \u{c9}chec de l'appel \u{e0} Ollama: {}", e);
                    (StatusCode::BAD_GATEWAY, "Erreur Ollama".to_string())
                })?;

            let status = ollama_res.status();
            let body = ollama_res.text().await.unwrap_or_default();

            if !status.is_success() {
                println!("\u{274c} Ollama a retourn\u{e9} une erreur {}: {}", status, body);
                return Err((StatusCode::BAD_GATEWAY, "Erreur dans la r\u{e9}ponse Ollama".to_string()));
            }

            let json_body: Value = serde_json::from_str(&body).unwrap_or_default();

            let answer = json_body["message"]["content"]
                .as_str()
                .unwrap_or("")
                .to_string();

            match scan_with_airs("".to_string(), answer.clone()).await {
                Ok(response_scan) if response_scan.action == "allow" => {
                    println!("\u{2705} R\u{e9}ponse autoris\u{e9}e par AIRS");
                    let resp = Response::builder().status(StatusCode::OK);
                    Ok(resp.body(Body::from(body)).map_err(|_| {
                        (StatusCode::INTERNAL_SERVER_ERROR, "Erreur de r\u{e9}ponse".to_string())
                    })?)
                }
                Ok(response_scan) => {
                    println!(
                        "\u{26d4} R\u{e9}ponse bloqu\u{e9}e par AIRS : {} (scan_id: {})",
                        response_scan.category, response_scan.scan_id
                    );
                    let blocked = json!({
                        "status": "blocked",
                        "message": "\u{26d4} R\u{e9}ponse bloqu\u{e9}e par la s\u{e9}curit\u{e9} AI Palo Alto Networks.",
                        "reason": response_scan.category,
                        "response_detected": response_scan.response_detected,
                        "suggestion": "Reformulez votre question pour \u{e9}viter le contenu inappropri\u{e9}.",
                        "scan_id": response_scan.scan_id,
                        "report_id": response_scan.report_id,
                    });
                    Ok(Response::builder()
                        .status(StatusCode::OK)
                        .body(Body::from(blocked.to_string()))
                        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Erreur r\u{e9}ponse bloqu\u{e9}e".to_string()))?)
                }
                Err(e) => {
                    println!("\u{274c} Erreur AIRS lors du scan de la r\u{e9}ponse : {}", e);
                    Err((StatusCode::INTERNAL_SERVER_ERROR, "Erreur scan r\u{e9}ponse".to_string()))
                }
            }
        }
        Ok(scan_result) => {
            println!(
                "\u{26d4} Prompt bloqu\u{e9} par AIRS : {} (scan_id: {})",
                scan_result.category, scan_result.scan_id
            );
            let blocked = json!({
                "status": "blocked",
                "message": "\u{26d4} Prompt bloqu\u{e9} par la s\u{e9}curit\u{e9} AI Palo Alto Networks.",
                "reason": scan_result.category,
                "prompt_detected": scan_result.prompt_detected,
                "suggestion": "Reformulez votre question pour \u{e9}viter le contenu inappropri\u{e9}.",
                "scan_id": scan_result.scan_id,
                "report_id": scan_result.report_id,
            });
            Ok(Response::builder()
                .status(StatusCode::OK)
                .body(Body::from(blocked.to_string()))
                .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Erreur r\u{e9}ponse bloqu\u{e9}e".to_string()))?)
        }
        Err(e) => {
            println!("\u{274c} Erreur AIRS lors du scan du prompt : {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Erreur scan prompt".to_string()))
        }
    }
}

async fn forward_to_ollama(mut req: Request) -> Result<Response, (StatusCode, String)> {
    let client = Client::new();

    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let query = req.uri().query().map(|q| format!("?{}", q)).unwrap_or_default();
    let full_path = format!("{}{}", path, query);

    println!("\u{1f535} Requ\u{ea}te entrante: [{}] {}", method, full_path);

    let url = format!("http://ollama:11434{}", full_path);

    let headers = req.headers().clone();
    let body = req.body_mut();
    let body_bytes = to_bytes(std::mem::take(body), 1024 * 1024)
        .await
        .map_err(|_| (StatusCode::BAD_REQUEST, "Failed to read body".to_string()))?;

    let mut request_builder = client.request(method.clone(), url.clone()).headers(headers);
    if !body_bytes.is_empty() {
        request_builder = request_builder.body(body_bytes);
    }

    let res = request_builder
        .send()
        .await
        .map_err(|e| {
            println!("\u{1f534} Forwarding vers Ollama KO: {}", e);
            (StatusCode::BAD_GATEWAY, "Erreur de forwarding".to_string())
        })?;

    let status = res.status();
    let mut response_builder = Response::builder().status(status);

    for (key, value) in res.headers() {
        response_builder = response_builder.header(key, value);
    }

    let stream_body = Body::from_stream(res.bytes_stream().map_ok(axum::body::Bytes::from));

    println!("\u{1f7e2} Forward OK [{}] -> {} ({})", method, full_path, status);

    Ok(response_builder.body(stream_body).map_err(|_| {
        (StatusCode::INTERNAL_SERVER_ERROR, "Erreur de construction de r\u{e9}ponse".to_string())
    })?)
}