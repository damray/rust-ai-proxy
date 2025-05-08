use std::env;
use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize)]
struct AiProfile {
    profile_id: String,
    profile_name: String,
}

#[derive(Debug, Serialize)]
struct Metadata {
    app_name: String,
    app_user: String,
    ai_model: String,
}

#[derive(Debug, Serialize)]
struct Content {
    prompt: String,
    response: String,
    code_prompt: String,
    code_response: String,
}

#[derive(Debug, Serialize)]
struct ScanRequest {
    tr_id: String,
    ai_profile: AiProfile,
    metadata: Metadata,
    contents: Vec<Content>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PromptDetection {
    pub dlp: bool,
    pub injection: bool,
    pub malicious_code: bool,
    pub toxic_content: bool,
    pub url_cats: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseDetection {
    pub dlp: bool,
    pub malicious_code: bool,
    pub toxic_content: bool,
    pub url_cats: bool,
}

#[derive(Debug, Deserialize)]
pub struct ScanResponse {
    pub action: String,
    pub category: String,
    pub profile_id: String,
    pub profile_name: String,
    pub prompt_detected: PromptDetection,
    pub response_detected: ResponseDetection,
    pub report_id: String,
    pub scan_id: String,
    pub tr_id: String,
}

/// Nettoie une chaîne vide ou whitespace, en remplaçant par "<vide>"
fn sanitize_field(s: &str) -> String {
    if s.trim().is_empty() {
        "<vide>".to_string()
    } else {
        s.to_string()
    }
}

pub async fn scan_with_airs(prompt: String, response: String) -> Result<ScanResponse> {
    let x_pan_token = env::var("PANW_X_PAN_TOKEN")?;
    let profile_id = env::var("PANW_PROFILE_ID")?;
    let profile_name = env::var("PANW_PROFILE_NAME")?;

    let tr_id = Uuid::new_v4().to_string();

    let payload = ScanRequest {
        tr_id: tr_id.clone(),
        ai_profile: AiProfile {
            profile_id,
            profile_name,
        },
        metadata: Metadata {
            app_name: "proxy".to_string(),
            app_user: "user_dam".to_string(),
            ai_model: "Ollama3".to_string(),
        },
        contents: vec![Content {
            prompt: sanitize_field(&prompt),
            response: sanitize_field(&response),
            code_prompt: "<vide>".to_string(),
            code_response: "<vide>".to_string(),
        }],
    };

    println!("🔍 Envoi vers AIRS (tr_id: {})...", tr_id);

    let client = Client::new();
    let res = client
        .post("https://service.api.aisecurity.paloaltonetworks.com/v1/scan/sync/request")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("x-pan-token", x_pan_token)
        .json(&payload)
        .send()
        .await;

    match res {
        Ok(r) => {
            let status = r.status();
            if !status.is_success() {
                let body = r.text().await.unwrap_or_default();
                println!("❌ Réponse AIRS NOK [{}]: {}", status, body);
                anyhow::bail!("AIRS scan failed ({}): {}", status, body);
            }
            println!("✅ Réponse AIRS OK [{}]", status);
            let parsed: ScanResponse = r.json().await?;
            Ok(parsed)
        }
        Err(e) => {
            println!("❌ Erreur d'envoi vers AIRS : {}", e);
            Err(anyhow::anyhow!(e))
        }
    }
}