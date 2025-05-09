use axum::{
    body::{to_bytes, Body},
    extract::Request,
    http::StatusCode,
    response::Response,
    routing::{post, any},
    Router,
};
use reqwest::Client;
use serde_json::json;
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;
use futures_util::stream::TryStreamExt;

mod airs;
use airs::scan_with_airs;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/api/chat", post(handle_unified_prompt))
        .fallback(any(forward_to_ollama))
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("🚀 Listening on {}", addr);

    axum::serve(tokio::net::TcpListener::bind(addr).await.unwrap(), app.into_make_service())
        .await
        .unwrap();
}

async fn handle_unified_prompt(mut req: Request) -> Result<Response, (StatusCode, String)> {
    let body = std::mem::take(req.body_mut());
    let body_bytes = to_bytes(body, 1024 * 1024)
        .await
        .map_err(|_| (StatusCode::BAD_REQUEST, "Failed to read body".to_string()))?;

    let original_body: serde_json::Value = serde_json::from_slice(&body_bytes)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid JSON".to_string()))?;

    // Extraire le prompt utilisateur
    let prompt = original_body["messages"]
        .as_array()
        .and_then(|msgs| msgs.iter().rev().find(|m| m["role"] == "user"))
        .and_then(|m| m["content"].as_str())
        .unwrap_or("")
        .to_string();

    println!("🧠 Unified prompt received: {}", prompt);

    // 🔍 Analyse du prompt
    match scan_with_airs(prompt.clone(),"N/A".to_string(),"N/A".to_string(),"N/A".to_string(),).await {
            Ok(scan_result) if scan_result.action == "allow" => {
            println!("✅ Prompt autorisé par AIRS");

            // ➡️ Forward vers Ollama
            let client = Client::new();
            let ollama_res = client
                .post("http://ollama:11434/api/chat")
                .header("Content-Type", "application/json")
                .body(body_bytes.clone())
                .send()
                .await
                .map_err(|e| {
                    println!("❌ Échec forward Ollama: {}", e);
                    (StatusCode::BAD_GATEWAY, "Erreur Ollama".to_string())
                })?;

            let status = ollama_res.status();
            let body = ollama_res.text().await.unwrap_or_default();

            if !status.is_success() {
                println!("❌ Ollama error [{}]: {}", status, body);
                return Err((StatusCode::BAD_GATEWAY, "Erreur dans la réponse Ollama".to_string()));
            }

            // 🔍 Analyse de la réponse Ollama
            let json_body: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
            let answer = json_body["message"]["content"]
                .as_str()
                .unwrap_or("")
                .to_string();

                match scan_with_airs( "N/A".to_string(), answer.clone(), "N/A".to_string(),"N/A".to_string(),).await {
                    Ok(resp_scan) if resp_scan.action == "allow" => {
                    println!("✅ Réponse autorisée par AIRS");
                    Ok(Response::builder()
                        .status(StatusCode::OK)
                        .body(Body::from(body))
                        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Erreur réponse".to_string()))?)
                }
                Ok(resp_scan) => {
                    println!("⛔ Réponse bloquée par AIRS: {}", resp_scan.category);
                    let msg = format!(
                        "⛔ Réponse bloquée par la sécurité AI Palo Alto Networks.\n\nCatégorie : {}\nSuggestion : reformulez votre question.",
                        resp_scan.category
                    );
                    let blocked_response = json!({
                        "message": {
                            "role": "assistant",
                            "content": msg
                        },
                        "done": true
                    });
                    Ok(Response::builder()
                        .status(StatusCode::OK)
                        .body(Body::from(blocked_response.to_string()))
                        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Erreur réponse bloquée".to_string()))?)
                }
                Err(e) => {
                    println!("❌ Erreur AIRS scan réponse: {}", e);
                    Err((StatusCode::INTERNAL_SERVER_ERROR, "Erreur scan réponse".to_string()))
                }
            }
        }
        Ok(scan_result) => {
            println!("⛔ Prompt bloqué par AIRS: {}", scan_result.category);
            let msg = format!(
                "⛔ Prompt bloqué par la sécurité AI Palo Alto Networks.\n\nCatégorie : {}\nSuggestion : reformulez votre question.",
                scan_result.category
            );
            let blocked_response = json!({
                "message": {
                    "role": "assistant",
                    "content": msg
                },
                "done": true
            });
            Ok(Response::builder()
                .status(StatusCode::OK)
                .body(Body::from(blocked_response.to_string()))
                .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Erreur réponse bloquée".to_string()))?)
        }
        Err(e) => {
            println!("❌ Erreur AIRS scan prompt: {}", e);
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

    println!("🔵 Incoming request: [{}] {}", method, full_path);
    let url = format!("http://ollama:11434{}", full_path);

    let headers = req.headers().clone();
    let body = std::mem::take(req.body_mut());
    let body_bytes = to_bytes(body, 1024 * 1024)
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
            println!("🔴 Erreur forward Ollama: {}", e);
            (StatusCode::BAD_GATEWAY, "Erreur vers Ollama".to_string())
        })?;

    let status = res.status();
    let mut response_builder = Response::builder().status(status);

    for (key, value) in res.headers() {
        response_builder = response_builder.header(key, value);
    }

    let stream_body = Body::from_stream(res.bytes_stream().map_ok(axum::body::Bytes::from));
    println!("🟢 Forwarded [{}] {} → {} ({})", method, full_path, url, status);

    Ok(response_builder
        .body(stream_body)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Erreur réponse finale".to_string()))?)
}