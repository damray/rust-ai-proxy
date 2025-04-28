use axum::{routing::post, Router, Json, response::IntoResponse, http::StatusCode};
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
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Listening on {}", addr);

    axum::serve(
        tokio::net::TcpListener::bind(addr).await.unwrap(),
        app.into_make_service()
    )
    .await
    .unwrap();
}

async fn handle_prompt(Json(payload): Json<PromptRequest>) -> impl IntoResponse {
    println!("Received prompt: {}", payload.prompt);

    // 1. Scanner le prompt avec AIRS
    match scan_with_airs(payload.prompt.clone(), "".to_string()).await {
        Ok(scan_result) => {
            if scan_result.action == "allow" {
                println!("Prompt autorisé, appel à Ollama...");

                // 2. Envoyer le prompt à Ollama
                match call_ollama(payload.prompt.clone()).await {
                    Ok(ollama_response) => {
                        println!("Réponse Ollama reçue, scan de la réponse...");

                        // 3. Scanner la réponse Ollama
                        match scan_with_airs("".to_string(), ollama_response.response.clone()).await {
                            Ok(response_scan_result) => {
                                if response_scan_result.action == "allow" {
                                    let reply = PromptResponse {
                                        response: ollama_response.response,
                                    };
                                    (StatusCode::OK, Json(reply))
                                } else {
                                    println!("Réponse bloquée par AIRS !");
                                    let report = json!({
                                        "status": "blocked",
                                        "reason": response_scan_result.category,
                                        "response_detected": response_scan_result.response_detected,
                                        "recommendations": [
                                            "La réponse générée a été bloquée pour raisons de sécurité.",
                                            "Merci de reformuler votre requête."
                                        ]
                                    });
                                    (StatusCode::FORBIDDEN, Json(report))
                                }
                            }
                            Err(e) => {
                                println!("Erreur scan AIRS réponse: {:?}", e);
                                let error = json!({
                                    "status": "error",
                                    "message": "Erreur lors du scan de la réponse AI."
                                });
                                (StatusCode::INTERNAL_SERVER_ERROR, Json(error))
                            }
                        }
                    }
                    Err(e) => {
                        println!("Erreur appel Ollama: {:?}", e);
                        let error = json!({
                            "status": "error",
                            "message": "Erreur pendant la génération AI."
                        });
                        (StatusCode::INTERNAL_SERVER_ERROR, Json(error))
                    }
                }
            } else {
                println!("Prompt bloqué par AIRS !");
                let report = json!({
                    "status": "blocked",
                    "reason": scan_result.category,
                    "prompt_detected": scan_result.prompt_detected,
                    "recommendations": [
                        "Merci de reformuler votre requête.",
                        "Évitez les contenus sensibles ou malicieux."
                    ]
                });
                (StatusCode::FORBIDDEN, Json(report))
            }
        }
        Err(e) => {
            println!("Erreur scan AIRS prompt: {:?}", e);
            let error = json!({
                "status": "error",
                "message": "Erreur de validation du prompt."
            });
            (StatusCode::INTERNAL_SERVER_ERROR, Json(error))
        }
    }
}
