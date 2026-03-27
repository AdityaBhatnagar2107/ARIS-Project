use reqwest::Client;
use serde_json::{json, Value};

pub async fn ask_ai(context: String, question: String) -> Result<String, Box<dyn std::error::Error + Send + Sync + 'static>> {
    // HARDCODED KEY: No more environment variable errors
    let clean_key = "AIzaSyA7aHVVKZLhXi7fJl3ha5cEMRcxrQNHVaA";
    
    let client = Client::new();

    let prompt_text = format!(
        "You are A.R.I.S, a codebase intelligence tool. Use the following context to answer.\n\nContext:\n{}\n\nQuestion: {}",
        context, question
    );

   // VERSION 1 STABLE: This is the most reliable endpoint
    let url = format!(
        "https://generativelanguage.googleapis.com/v1/models/gemini-2.5-flash:generateContent?key={}",
        clean_key
    );

    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&json!({
            "contents": [{
                "parts": [{
                    "text": prompt_text
                }]
            }]
        }))
        .send()
        .await?;
    if !response.status().is_success() {
        let status = response.status();
        let error_body = response.text().await.unwrap_or_default();
        return Err(format!("API Error {}: {}", status, error_body).into());
    }

    let body: Value = response.json().await?;
    
    let text = body["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .ok_or("Failed to parse Gemini response text")?;

    Ok(text.to_string())
}