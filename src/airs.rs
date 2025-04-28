use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use uuid::Uuid;

/// Structure pour envoyer une requête à AIRS
#[derive(Serialize, Debug)]
struct ScanRequest {
    tr_id: String,
    ai_profile: AiProfile,
    metadata: Metadata,
    contents: Vec<Content>,
}

#[derive(Serialize, Debug)]
struct AiProfile {
    profile_id: String,
    profile_name: String,
}

#[derive(Serialize, Debug)]
struct Metadata {
    app_name: String,
    app_user: String,
    ai_model: String,
}

#[derive(Serialize, Debug)]
struct Content {
    prompt: String,
    response: String,
    code_prompt: String,
    code_response: String,
}

/// Structure de réponse de AIRS
#[derive(Debug, Deserialize)]
pub struct ScanResponse {
    pub action: String,
    pub category: String,
    pub profile_id: String,
    pub profile_name: String,
    pub prompt_detected: PromptDetection,
    pub report_id: String,
    pub response_detected: ResponseDetection,
    pub scan_id: String,
    pub tr_id: String,
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

/// Fonction qui appelle AIRS
pub async fn scan_with_airs(prompt: String, response: String) -> Result<ScanResponse> {
    let x_pan_token = env::var("PANW_X_PAN_TOKEN")?;
    let profile_id = env::var("PANW_PROFILE_ID")?;
    let profile_name = env::var("PANW_PROFILE_NAME")?;

    let tr_id = Uuid::new_v4().to_string();

    let payload = ScanRequest {
        tr_id,
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
            prompt,
            response: "String".to_string(),
            code_prompt: "String".to_string(),
            code_response: "String".to_string(),
        }],
    };

    let pretty_payload = serde_json::to_string_pretty(&payload)?;
    println!("📦 Payload envoyé à AIRS :\n{}", pretty_payload);

    let client = Client::new();

    let res = client
        .post("https://service.api.aisecurity.paloaltonetworks.com/v1/scan/sync/request")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("x-pan-token", x_pan_token)
        .json(&payload)
        .send()
        .await?
        .json::<ScanResponse>()
        .await?;

    Ok(res)
}
