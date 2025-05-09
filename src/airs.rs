use serde::{Deserialize, Serialize};
use std::env;
use uuid::Uuid;
use reqwest::Client;

#[derive(Debug, Deserialize)]
pub struct ScanResponse {
    pub action: String,
    pub category: String,
    pub prompt_detected: PromptDetection,
    pub response_detected: ResponseDetection,
    pub report_id: String,
    pub scan_id: String,
    pub tr_id: String,
    pub profile_id: String,
    pub profile_name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PromptDetection {
    pub dlp: bool,
    pub injection: bool,
    pub malicious_code: bool,
    pub toxic_content: bool,
    pub url_cats: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ResponseDetection {
    pub dlp: bool,
    pub malicious_code: bool,
    pub toxic_content: bool,
    pub url_cats: bool,
}

#[derive(Serialize)]
struct ScanRequest {
    tr_id: String,
    ai_profile: AiProfile,
    metadata: Metadata,
    contents: Vec<Content>,
}

#[derive(Serialize)]
struct AiProfile {
    profile_id: String,
    profile_name: String,
}

#[derive(Serialize)]
struct Metadata {
    app_name: String,
    app_user: String,
    ai_model: String,
}

#[derive(Serialize)]
struct Content {
    prompt: String,
    response: String,
    code_prompt: String,
    code_response: String,
}

pub async fn scan_with_airs(prompt: String, response: String, code_prompt: String, code_response: String) -> Result<ScanResponse, String> {
    let x_pan_token = env::var("PANW_X_PAN_TOKEN").map_err(|e| e.to_string())?;
    let profile_id = env::var("PANW_PROFILE_ID").map_err(|e| e.to_string())?;
    let profile_name = env::var("PANW_PROFILE_NAME").map_err(|e| e.to_string())?;
    let tr_id = Uuid::new_v4().to_string();

    println!("🔍 Envoi vers AIRS (tr_id: {})...", tr_id);

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
            prompt,
            response: if response.trim().is_empty() {
                "N/A".to_string()
            } else {
                response
            },
            code_prompt: if code_prompt.trim().is_empty() {
                "N/A".to_string()
            } else {
                code_prompt
            },
            code_response: if code_response.trim().is_empty() {
                "N/A".to_string()
            } else {
                code_response
            },
        }],
    };

    let client = Client::new();
    let res = client
        .post("https://service.api.aisecurity.paloaltonetworks.com/v1/scan/sync/request")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("x-pan-token", x_pan_token)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Erreur HTTP: {}", e))?;

    let status = res.status();
    let body = res.text().await.map_err(|e| format!("Erreur body: {}", e))?;

    if status != 200 {
        println!("❌ Réponse AIRS NOK [{}]: {}", status, body);
        return Err(format!("Erreur AIRS: {}", body));
    }

    let parsed: ScanResponse = serde_json::from_str(&body)
        .map_err(|e| format!("Erreur parsing JSON AIRS: {}", e))?;

    println!("✅ Réponse AIRS OK [{}]", status);
    Ok(parsed)
}