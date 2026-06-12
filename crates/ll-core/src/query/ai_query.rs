/// Translates a natural-language query into a QueryFilter using AI.
/// Falls back to text-match if AI is unavailable.
use crate::models::query::QueryFilter;

pub async fn translate(
    nl_query: &str,
    ai_base_url: &str,
    api_key: &str,
    model: &str,
) -> QueryFilter {
    let prompt = format!(
        r#"Convert this natural language log query to a JSON QueryFilter object.

Query: "{}"

Respond ONLY with a JSON object matching this schema (all fields optional):
{{
  "text": "full-text search string",
  "levels": ["error", "warn", "fatal"],
  "services": ["service-name"],
  "from": "ISO8601 timestamp",
  "to": "ISO8601 timestamp",
  "has_stacktrace": true,
  "cluster_id": "cluster-id"
}}

Do not include fields that are not relevant to the query."#,
        nl_query
    );

    let body = serde_json::json!({
        "model": model,
        "max_tokens": 512,
        "messages": [{"role": "user", "content": prompt}]
    });

    let result = reqwest::Client::new()
        .post(format!("{}/v1/messages", ai_base_url.trim_end_matches('/')))
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await;

    if let Ok(resp) = result {
        if let Ok(json) = resp.json::<serde_json::Value>().await {
            let text = json["content"][0]["text"].as_str().unwrap_or("");
            // Extract JSON block from response
            let json_str = extract_json(text);
            if let Ok(filter) = serde_json::from_str::<QueryFilter>(&json_str) {
                return filter;
            }
        }
    }

    // Fallback: use the raw query as text search
    QueryFilter { text: Some(nl_query.to_string()), ..Default::default() }
}

fn extract_json(s: &str) -> String {
    if let (Some(start), Some(end)) = (s.find('{'), s.rfind('}')) {
        s[start..=end].to_string()
    } else {
        s.to_string()
    }
}
