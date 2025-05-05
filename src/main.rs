use axum::{
    body::{to_bytes, Body},
    extract::{Json, Request},
    http::StatusCode,
    response::Response,
    routing::{any, post},
    Router,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;

mod airs;
mod ollama;

use airs::scan_with_airs;
use ollama::call_ollama;
use futures_util::stream::TryStreamExt; 

#[derive(Debug, Deserialize)]
struct PromptRequest {
    prompt: String,
}

#[derive(Debug, Serialize)]
struct PromptResponse {
    response: String,
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/prompt", post(handle_prompt)) // Spécial: route protégée
        .fallback(any(forward_to_ollama)) // TOUS les autres chemins => forward
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("🚀 Listening on {}", addr);

    axum::serve(
        tokio::net::TcpListener::bind(addr).await.unwrap(),
        app.into_make_service(),
    )
    .await
    .unwrap();
}

async fn handle_prompt(
    Json(payload): Json<PromptRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    println!("Received prompt: {}", payload.prompt);

    match scan_with_airs(payload.prompt.clone(), "".to_string()).await {
        Ok(scan_result) => {
            if scan_result.action == "allow" {
                match call_ollama(payload.prompt.clone()).await {
                    Ok(ollama_response) => {
                        match scan_with_airs("".to_string(), ollama_response.response.clone()).await {
                            Ok(response_scan_result) => {
                                if response_scan_result.action == "allow" {
                                    (
                                        StatusCode::OK,
                                        Json(json!({
                                            "response": ollama_response.response
                                        })),
                                    )
                                } else {
                                    (
                                        (
                                            StatusCode::OK,
                                            Json(json!({
                                                "status": "blocked",
                                                "message": "⛔ Réponse bloquée par la sécurité AI Palo Alto Networks.",
                                                "reason": response_scan_result.category,
                                                "suggestion": "Reformulez votre question pour éviter le contenu inapproprié.",
                                                "response_detected": response_scan_result.response_detected
                                            })),
                                        )
                                    )
                                }
                            }
                            Err(_) => (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(json!({
                                    "status": "error",
                                    "message": "Erreur lors du scan de la réponse AI."
                                })),
                            ),
                        }
                    }
                    Err(_) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({
                            "status": "error",
                            "message": "Erreur lors de l'appel à Ollama."
                        })),
                    ),
                }
            } else {
                (
                    StatusCode::FORBIDDEN,
                    Json(json!({
                        "status": "blocked",
                        "reason": scan_result.category,
                        "prompt_detected": scan_result.prompt_detected,
                    })),
                )
            }
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "status": "error",
                "message": "Erreur lors du scan du prompt."
            })),
        ),
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
            println!("🔴 Error sending request to Ollama: {}", e);
            (StatusCode::BAD_GATEWAY, "Failed to forward to Ollama".to_string())
        })?;

    let status = res.status();
    let mut response_builder = Response::builder().status(status);

    for (key, value) in res.headers() {
        response_builder = response_builder.header(key, value);
    }
    let stream_body = Body::from_stream(res.bytes_stream().map_ok(axum::body::Bytes::from));
    
    println!("🟢 Forwarded [{}] {} -> {} ({})", method, full_path, url, status);

    Ok(response_builder
        .body(stream_body)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to build response".to_string()))?)
}

