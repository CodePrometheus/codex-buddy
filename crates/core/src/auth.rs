use std::fs;
use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde_json::Value;

use crate::error::{Error, Result};

const AUTH_CLAIM: &str = "https://api.openai.com/auth";
const PROFILE_CLAIM: &str = "https://api.openai.com/profile";

/// Account identity and display metadata parsed from auth.json.
#[derive(Debug, Clone)]
pub struct AuthInfo {
    /// Unique key: `chatgpt_user_id::chatgpt_account_id`.
    pub account_key: String,
    pub email: Option<String>,
    /// ChatGPT plan type (free / plus / pro / business, ...).
    pub plan: Option<String>,
    pub chatgpt_user_id: Option<String>,
    pub chatgpt_account_id: Option<String>,
}

/// Read and parse an auth.json file.
pub fn load_auth_info(path: &Path) -> Result<AuthInfo> {
    let data = fs::read_to_string(path)?;
    let auth: Value = serde_json::from_str(&data)?;
    parse_auth_info(&auth)
}

/// Parse account info from an already-parsed auth.json value.
///
/// Only ChatGPT logins are supported (requires `tokens.id_token`); API-key mode is out of scope.
pub fn parse_auth_info(auth: &Value) -> Result<AuthInfo> {
    let id_token = auth
        .get("tokens")
        .and_then(|t| t.get("id_token"))
        .and_then(Value::as_str)
        .ok_or_else(|| Error::InvalidAuth("missing tokens.id_token (ChatGPT login only)".into()))?;

    let claims = decode_jwt_claims(id_token)?;
    let auth_claim = claims.get(AUTH_CLAIM);

    let chatgpt_user_id = auth_claim
        .and_then(|a| a.get("chatgpt_user_id").or_else(|| a.get("user_id")))
        .and_then(Value::as_str)
        .map(str::to_owned);

    // Prefer the id_token's chatgpt_account_id; fall back to tokens.account_id.
    let chatgpt_account_id = auth_claim
        .and_then(|a| a.get("chatgpt_account_id"))
        .and_then(Value::as_str)
        .map(str::to_owned)
        .or_else(|| {
            auth.get("tokens")
                .and_then(|t| t.get("account_id"))
                .and_then(Value::as_str)
                .map(str::to_owned)
        });

    let plan = auth_claim
        .and_then(|a| a.get("chatgpt_plan_type"))
        .and_then(Value::as_str)
        .map(str::to_owned);

    let email = claims
        .get("email")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .or_else(|| {
            claims
                .get(PROFILE_CLAIM)
                .and_then(|p| p.get("email"))
                .and_then(Value::as_str)
                .map(str::to_owned)
        });

    let account_key = match (&chatgpt_user_id, &chatgpt_account_id) {
        (Some(u), Some(a)) => format!("{u}::{a}"),
        _ => {
            return Err(Error::InvalidAuth(
                "cannot derive account key (missing chatgpt_user_id or chatgpt_account_id)".into(),
            ));
        }
    };

    Ok(AuthInfo {
        account_key,
        email,
        plan,
        chatgpt_user_id,
        chatgpt_account_id,
    })
}

/// Decode a JWT's payload segment into JSON claims. Does not verify the signature.
pub fn decode_jwt_claims(id_token: &str) -> Result<Value> {
    let mut parts = id_token.split('.');
    let _header = parts.next();
    let payload = parts
        .next()
        .filter(|p| !p.is_empty())
        .ok_or_else(|| Error::InvalidAuth("malformed JWT (no payload segment)".into()))?;

    let payload = payload.trim_end_matches('=');
    let bytes = URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|e| Error::InvalidAuth(format!("JWT payload base64 decode failed: {e}")))?;
    let claims: Value = serde_json::from_slice(&bytes)
        .map_err(|e| Error::InvalidAuth(format!("JWT payload is not valid JSON: {e}")))?;
    Ok(claims)
}

#[cfg(test)]
mod tests;
