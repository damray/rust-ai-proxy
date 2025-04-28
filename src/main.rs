use axum::body::to_bytes;
use axum::{
    body::{Body, Bytes},
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
        .route("/prompt", post(handle_prompt))
        .route("/v1/*path", any(forward_to_ollama)) // <-- Ajoute ce forwarder général
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Listening on {}", addr);

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
                        match scan_with_airs("".to_string(), ollama_response.response.clone()).await
                        {
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
                                        StatusCode::FORBIDDEN,
                                        Json(json!({
                                            "status": "blocked",
                                            "reason": response_scan_result.category,
                                            "response_detected": response_scan_result.response_detected,
                                        })),
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

// Fonction de forwarding pour tout le reste
async fn forward_to_ollama(mut req: Request) -> Result<Response, (StatusCode, String)> {
    let client = Client::new();

    // Construit la nouvelle URL vers Ollama
    let uri = req
        .uri()
        .path_and_query()
        .map(|x| x.as_str())
        .unwrap_or("/");
    let url = format!("http://127.0.0.1:11434{}", uri); // adapte selon ton Ollama

    println!("Forwarding request to {}", url);

    // Construit la nouvelle requête
    let method = req.method().clone();
    let headers = req.headers().clone();
    let body = req.body_mut();
    let body_bytes = to_bytes(std::mem::take(body), 1024 * 1024) // 1MB max
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to read body".to_string(),
            )
        })?;

    let mut request_builder = client.request(method, url).headers(headers);

    if !body_bytes.is_empty() {
        request_builder = request_builder.body(body_bytes);
    }

    // Envoie la requête vers Ollama
    let res = request_builder
        .send()
        .await
        .map_err(|_| (StatusCode::BAD_GATEWAY, "Failed to forward".to_string()))?;

    // Construit la réponse à retourner vers OpenWebUI
    let mut response_builder = axum::response::Response::builder().status(res.status());

    for (key, value) in res.headers() {
        response_builder = response_builder.header(key, value);
    }

    let body = res.bytes().await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to read response body".to_string(),
        )
    })?;
    Ok(response_builder.body(Body::from(body)).unwrap())
}
