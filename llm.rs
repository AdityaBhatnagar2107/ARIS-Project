use serde::{Deserialize, Serialize};
use reqwest::Client;

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<Message>,
    max_tokens: u32,
}

#[derive(Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

// THIS IS THE LINE THE COMPILER IS MISSING
pub async fn ask_ai(context: String, question: String) -> Result<String, Box<dyn std::error::Error>> {
    let client = Client::new();
    let api_key = std::env::var("ANTHROPIC_API_KEY").map_err(|_| "ANTHROPIC_API_KEY not set")?;
    
    let prompt = format!("Codebase Context:\n{}\n\nQuestion: {}", context, question);

    let response = client.post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&AnthropicRequest {
            model: "claude-3-sonnet-20240229".to_string(),
            messages: vec![Message { role: "user".to_string(), content: prompt }],
            max_tokens: 1024,
        })
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    let answer = response["content"][0]["text"]
        .as_str()
        .ok_or("No text in response")?
        .to_string();

    Ok(answer)
}