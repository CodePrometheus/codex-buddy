use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;

/// Build a codex auth.json string with the given identity (id_token is an unsigned JWT).
pub fn auth_json(user: &str, acct: &str) -> String {
    let claims = serde_json::json!({
        "email": format!("{user}@example.com"),
        "https://api.openai.com/auth": {
            "chatgpt_user_id": user,
            "chatgpt_account_id": acct,
            "chatgpt_plan_type": "pro"
        }
    });
    let payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).unwrap());
    serde_json::json!({
        "auth_mode": "chatgpt",
        "tokens": { "id_token": format!("h.{payload}.s"), "account_id": acct }
    })
    .to_string()
}
