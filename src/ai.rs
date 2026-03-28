use reqwest::Client;
use serde_json::json;
use std::env;
use std::time::Duration;

pub async fn ask_gemini(
    context: String,
    question: String,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {

    // 🔐 SAFE ENV KEY
    let api_key = env::var("GOOGLE_API_KEY")
        .expect("GOOGLE_API_KEY not set");

    // ⚡ FAST + RELIABLE CLIENT
    let client = Client::builder()
        .timeout(Duration::from_secs(12))
        .build()?;

    // 🚀 MODEL ENDPOINT
    let url = format!(
        "https://generativelanguage.googleapis.com/v1/models/gemini-2.5-flash:generateContent?key={}",
        api_key
    );

    // ⚡ LIMIT CONTEXT (VERY IMPORTANT FOR SPEED)
    let trimmed_context: String = context.chars().take(1200).collect();

    let prompt = format!(
        "You are A.R.I.S (Autonomous Repository Intelligence System).\n\
        Analyze the given codebase context and answer precisely.\n\n\
        CONTEXT:\n{}\n\n\
        QUESTION:\n{}\n\n\
        RULES:\n\
        - Be concise (max 4 lines)\n\
        - Be technical\n\
        - Refer to actual file/function names\n\
        - Do NOT hallucinate",
        trimmed_context, question
    );

    let body = json!({
        "contents": [{
            "parts": [{ "text": prompt }]
        }],
        "generationConfig": {
            "temperature": 0.3,
            "maxOutputTokens": 200
        }
    });

    // ===== API CALL =====
    let res = client.post(&url).json(&body).send().await?;

    // ❗ HANDLE HTTP ERRORS CLEANLY
    if !res.status().is_success() {
        let err_text = res.text().await.unwrap_or("Unknown error".to_string());
        return Ok(format!("⚠️ API ERROR:\n{}", err_text));
    }

    let raw = res.text().await?;
    println!("RAW: {}", raw);

    // ===== SAFE PARSE =====
    let parsed: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(_) => {
            return Ok(format!("⚠️ Invalid JSON response:\n{}", raw));
        }
    };

    // ===== SAFE EXTRACTION =====
    let answer = parsed["candidates"]
        .get(0)
        .and_then(|c| c["content"]["parts"].get(0))
        .and_then(|p| p["text"].as_str())
        .unwrap_or("⚠️ No response generated.")
        .to_string();

    println!("ANSWER: {}", answer);

    Ok(answer)
}