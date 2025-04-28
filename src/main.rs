use axum::{routing::post, Router, Json, serve};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;

// Structures pour JSON
#[derive(Debug, Deserialize)]
struct PromptRequest {
    prompt: String,
}

#[derive(Debug, Serialize)]
struct PromptResponse {
    response: String,
}

// Handler pour /prompt
async fn handle_prompt(Json(payload): Json<PromptRequest>) -> Json<PromptResponse> {
    println!("Received prompt: {}", payload.prompt);

    let reply = PromptResponse {
        response: format!("You said: {}", payload.prompt),
    };

    Json(reply)
}

#[tokio::main]
async fn main() {
    // Définir les routes
    let app = Router::new()
        .route("/prompt", post(handle_prompt))
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Listening on {}", addr);

    // **Nouvelle manière** : utiliser serve()
    axum::serve(
        tokio::net::TcpListener::bind(addr).await.unwrap(),
        app.into_make_service()
    )
    .await
    .unwrap();
}
