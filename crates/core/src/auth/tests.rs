use super::*;
use serde_json::json;

fn make_jwt(claims: &Value) -> String {
    let payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(claims).unwrap());
    format!("aGVhZGVy.{payload}.c2ln")
}

#[test]
fn parses_full_chatgpt_auth() {
    let jwt = make_jwt(&json!({
        "email": "user@example.com",
        AUTH_CLAIM: {
            "chatgpt_user_id": "user-123",
            "chatgpt_account_id": "acct-456",
            "chatgpt_plan_type": "pro"
        }
    }));
    let auth = json!({
        "auth_mode": "chatgpt",
        "tokens": { "id_token": jwt, "account_id": "acct-456" }
    });
    let info = parse_auth_info(&auth).unwrap();
    assert_eq!(info.account_key, "user-123::acct-456");
    assert_eq!(info.email.as_deref(), Some("user@example.com"));
    assert_eq!(info.plan.as_deref(), Some("pro"));
}

#[test]
fn email_falls_back_to_profile_claim() {
    let jwt = make_jwt(&json!({
        PROFILE_CLAIM: { "email": "p@example.com" },
        AUTH_CLAIM: { "chatgpt_user_id": "u", "chatgpt_account_id": "a" }
    }));
    let auth = json!({ "tokens": { "id_token": jwt } });
    let info = parse_auth_info(&auth).unwrap();
    assert_eq!(info.email.as_deref(), Some("p@example.com"));
    assert_eq!(info.account_key, "u::a");
}

#[test]
fn account_id_falls_back_to_tokens() {
    let jwt = make_jwt(&json!({ AUTH_CLAIM: { "chatgpt_user_id": "u" } }));
    let auth = json!({ "tokens": { "id_token": jwt, "account_id": "acct-from-tokens" } });
    let info = parse_auth_info(&auth).unwrap();
    assert_eq!(info.account_key, "u::acct-from-tokens");
}

#[test]
fn missing_id_token_is_error() {
    let auth = json!({ "auth_mode": "apikey", "OPENAI_API_KEY": "sk-xxx" });
    assert!(parse_auth_info(&auth).is_err());
}

#[test]
fn missing_identity_is_error() {
    let jwt = make_jwt(&json!({ "email": "x@example.com" }));
    let auth = json!({ "tokens": { "id_token": jwt } });
    assert!(parse_auth_info(&auth).is_err());
}

#[test]
fn malformed_jwt_is_error() {
    let auth = json!({ "tokens": { "id_token": "not-a-jwt" } });
    assert!(parse_auth_info(&auth).is_err());
}
