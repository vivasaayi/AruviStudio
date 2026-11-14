use axum::http::HeaderMap;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use hmac::{Hmac, Mac};
use sha1::Sha1;
use std::collections::HashMap;

pub(crate) fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub(crate) fn messaging_twiml(message: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><Response><Message>{}</Message></Response>"#,
        xml_escape(message)
    )
}

pub(crate) fn voice_gather_twiml(prompt: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Response>
  <Say>{}</Say>
  <Gather input="speech" action="/webhooks/twilio/voice/gather" method="POST" speechTimeout="auto" />
  <Redirect method="POST">/webhooks/twilio/voice</Redirect>
</Response>"#,
        xml_escape(prompt)
    )
}

pub(crate) fn planner_reply_text(
    response: &crate::services::planner_service::PlannerTurnResponse,
) -> String {
    let mut parts = vec![response.assistant_message.clone()];
    if !response.execution_lines.is_empty() {
        parts.extend(response.execution_lines.clone());
    }
    if !response.execution_errors.is_empty() {
        parts.push(format!("Errors: {}", response.execution_errors.join(" | ")));
    }
    if response.status == "proposal" {
        parts.push("Reply confirm to apply the proposal.".to_string());
    }
    parts.join("\n")
}

pub(crate) fn validate_twilio_signature(
    auth_token: Option<&str>,
    base_url: Option<&str>,
    path: &str,
    params: &HashMap<String, String>,
    headers: &HeaderMap,
) -> Result<(), String> {
    let auth_token = auth_token.filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "Twilio webhook authentication is not configured".to_string())?;
    let base_url = base_url.filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "Twilio webhook URL is not configured".to_string())?;
    let Some(signature) = headers
        .get("X-Twilio-Signature")
        .and_then(|value| value.to_str().ok())
    else {
        return Err("Missing X-Twilio-Signature header".to_string());
    };

    let mut data = format!("{}{}", base_url.trim_end_matches('/'), path);
    let mut sorted = params.iter().collect::<Vec<_>>();
    sorted.sort_by(|left, right| left.0.cmp(right.0));
    for (key, value) in sorted {
        data.push_str(key);
        data.push_str(value);
    }

    let mut mac =
        Hmac::<Sha1>::new_from_slice(auth_token.as_bytes()).map_err(|error| error.to_string())?;
    mac.update(data.as_bytes());
    let supplied = STANDARD.decode(signature).map_err(|_| "Invalid Twilio signature".to_string())?;
    mac.verify_slice(&supplied).map_err(|_| "Invalid Twilio signature".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn webhook_signature_requires_configuration_and_valid_signature() {
        let params = HashMap::from([("Body".to_string(), "hello".to_string()), ("From".to_string(), "whatsapp:+15550000000".to_string())]);
        let mut headers = HeaderMap::new();
        let validate = |token, url, headers: &HeaderMap| validate_twilio_signature(token, url, "/webhooks/twilio/whatsapp", &params, headers);
        assert!(validate(None, None, &headers).is_err());
        assert!(validate(Some(" "), Some("https://example.test"), &headers).is_err());
        assert!(validate(Some("secret"), Some(""), &headers).is_err());
        assert!(validate(Some("secret"), Some("https://example.test"), &headers).is_err());
        let mut mac = Hmac::<Sha1>::new_from_slice(b"secret").unwrap();
        mac.update(b"https://example.test/webhooks/twilio/whatsappBodyhelloFromwhatsapp:+15550000000");
        headers.insert("X-Twilio-Signature", HeaderValue::from_str(&STANDARD.encode(mac.finalize().into_bytes())).unwrap());
        assert!(validate(Some("secret"), Some("https://example.test/"), &headers).is_ok());
        assert!(validate(Some("wrong"), Some("https://example.test"), &headers).is_err());
        headers.insert("X-Twilio-Signature", HeaderValue::from_static("not-base64!"));
        assert!(validate(Some("secret"), Some("https://example.test"), &headers).is_err());
    }
}
