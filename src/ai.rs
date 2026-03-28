use reqwest::Client;
use serde_json::json;

pub async fn ask_gemini(
    context: String,
    question: String,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {

    // 🔐 USE ENV KEY (safe)
    let api_key = std::env::var("GOOGLE_API_KEY")
        .expect("Set GOOGLE_API_KEY first");

    // ⚡ FAST CLIENT
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    // 🚀 FAST MODEL
    let url = format!(
        "https://generativelanguage.googleapis.com/v1/models/gemini-2.5-flash:generateContent?key={}",
        api_key
    );

    // ⚡ LIMIT CONTEXT (VERY IMPORTANT)
    let trimmed_context: String = context.chars().take(1200).collect();

    let prompt = format!(
        "You are A.R.I.S.\n\nContext:\n{}\n\nQuestion:\n{}\n\nAnswer in 3-4 lines, technical.",
        trimmed_context, question
    );

    let body = json!({
        "contents": [{
            "parts": [{ "text": prompt }]
        }]
    });

    // ===== API CALL =====
    let res = client.post(&url).json(&body).send().await?;

    let raw = res.text().await?;
    println!("RAW: {}", raw);

    // ===== PARSE =====
    let v: serde_json::Value = serde_json::from_str(&raw)?;

    let answer = v["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .unwrap_or("No response from AI.")
        .to_string();

    println!("ANSWER: {}", answer);

    Ok(answer)
}