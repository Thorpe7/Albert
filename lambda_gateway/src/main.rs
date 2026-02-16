use aws_sdk_lambda::primitives::Blob;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use lambda_http::{run, service_fn, Request, Response, Body};
use serde_json::Value;
use std::env;

fn verify_signature(public_key_hex: &str, signature_hex: &str, timestamp: &str, body: &str) -> bool {
    let Ok(pub_key_bytes) = hex::decode(public_key_hex) else {
        return false;
    };
    let Ok(pub_key_array): Result<[u8; 32], _> = pub_key_bytes.try_into() else {
        return false;
    };
    let Ok(verifying_key) = VerifyingKey::from_bytes(&pub_key_array) else {
        return false;
    };

    let Ok(sig_bytes) = hex::decode(signature_hex) else {
        return false;
    };
    let Ok(sig_array): Result<[u8; 64], _> = sig_bytes.try_into() else {
        return false;
    };
    let signature = Signature::from_bytes(&sig_array);

    let message = format!("{}{}", timestamp, body);
    verifying_key.verify(message.as_bytes(), &signature).is_ok()
}

fn json_response(status: u16, body: &Value) -> Result<Response<Body>, lambda_http::Error> {
    Ok(Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Body::Text(body.to_string()))
        .unwrap())
}

async fn handler(event: Request) -> Result<Response<Body>, lambda_http::Error> {
    let skip_verify = env::var("SKIP_SIGNATURE_VERIFY")
        .map(|v| v == "true")
        .unwrap_or(false);

    let body_str = match event.body() {
        Body::Text(s) => s.clone(),
        Body::Binary(b) => String::from_utf8_lossy(b).to_string(),
        Body::Empty => String::new(),
    };

    if !skip_verify {
        let public_key = env::var("DISCORD_PUBLIC_KEY")
            .expect("DISCORD_PUBLIC_KEY env var required");

        let signature = event.headers()
            .get("x-signature-ed25519")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        let timestamp = event.headers()
            .get("x-signature-timestamp")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if !verify_signature(&public_key, signature, timestamp, &body_str) {
            tracing::warn!("Invalid signature");
            return json_response(401, &serde_json::json!({"error": "Invalid signature"}));
        }
    }

    let payload: Value = serde_json::from_str(&body_str)
        .unwrap_or_default();

    let interaction_type = payload["type"].as_u64().unwrap_or(0);

    // Type 1: Ping
    if interaction_type == 1 {
        tracing::info!("Responding to Discord Ping");
        return json_response(200, &serde_json::json!({"type": 1}));
    }

    // Type 2: ApplicationCommand
    if interaction_type == 2 {
        let command_name = payload["data"]["name"].as_str().unwrap_or("");
        tracing::info!(command = command_name, "Received application command");

        // Invoke Lambda B asynchronously (fire-and-forget)
        let worker_function = env::var("WORKER_FUNCTION_NAME")
            .expect("WORKER_FUNCTION_NAME env var required");

        let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let lambda_client = aws_sdk_lambda::Client::new(&config);

        let invoke_result = lambda_client
            .invoke()
            .function_name(&worker_function)
            .invocation_type(aws_sdk_lambda::types::InvocationType::Event)
            .payload(Blob::new(body_str.as_bytes()))
            .send()
            .await;

        if let Err(e) = invoke_result {
            tracing::error!(error = %e, "Failed to invoke worker Lambda");
        }

        // Return deferred response
        // Context menu ("Summarize Article") → visible; slash commands → ephemeral
        let is_ephemeral = matches!(command_name, "summary-24hr" | "summary-peruser" | "summary-article");

        let deferred = if is_ephemeral {
            serde_json::json!({"type": 5, "data": {"flags": 64}})
        } else {
            serde_json::json!({"type": 5})
        };

        return json_response(200, &deferred);
    }

    tracing::warn!(interaction_type, "Unhandled interaction type");
    json_response(400, &serde_json::json!({"error": "Unhandled interaction type"}))
}

#[tokio::main]
async fn main() -> Result<(), lambda_http::Error> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    run(service_fn(handler)).await
}
