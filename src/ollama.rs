use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
}

#[derive(Debug, Deserialize)]
pub struct OllamaResponse {
    pub response: String,
}

/// Fonction pour appeler Ollama localement
pub async fn call_ollama(prompt: String) -> Result<OllamaResponse> {
    let client = Client::new();

    let request_body = OllamaRequest {
        model: "ollama3".to_string(),
        prompt,
    };

    let res = client
        .post("http://localhost:11434/api/generate")
        .json(&request_body)
        .send()
        .await?
        .json::<OllamaResponse>()
        .await?;

    Ok(res)
}
