//! Discord HTTP Interactions support.
//!
//! Implements Ed25519 signature verification (required by Discord) and
//! slash command registration at startup.

use anyhow::{Context, Result};
use ed25519_dalek::{Signature, VerifyingKey, Verifier};

// ---------------------------------------------------------------------------
// Signature verification
// ---------------------------------------------------------------------------

/// Verify a Discord Ed25519 interaction signature.
///
/// Discord sends the signature in the `X-Signature-Ed25519` header and the
/// timestamp in `X-Signature-Timestamp`. The signed message is the
/// concatenation of the timestamp and the raw request body.
///
/// Returns `true` if the signature is valid.
pub fn verify_signature(public_key_hex: &str, signature_hex: &str, timestamp: &str, body: &[u8]) -> bool {
    let Ok(pubkey_bytes) = hex::decode(public_key_hex) else {
        return false;
    };
    let Ok(pubkey_array): Result<[u8; 32], _> = pubkey_bytes.try_into() else {
        return false;
    };
    let Ok(verifying_key) = VerifyingKey::from_bytes(&pubkey_array) else {
        return false;
    };

    let Ok(sig_bytes) = hex::decode(signature_hex) else {
        return false;
    };
    let Ok(sig_array): Result<[u8; 64], _> = sig_bytes.try_into() else {
        return false;
    };
    let signature = Signature::from_bytes(&sig_array);

    // Message = timestamp_bytes || body_bytes
    let mut message = timestamp.as_bytes().to_vec();
    message.extend_from_slice(body);

    verifying_key.verify(&message, &signature).is_ok()
}

// ---------------------------------------------------------------------------
// Command registration
// ---------------------------------------------------------------------------

/// Register the `/faucet` slash command with Discord's REST API.
///
/// This is called once at startup when Discord credentials are configured.
/// Uses a global (cross-guild) command so it works in any server.
pub async fn register_commands(token: &str, app_id: &str) -> Result<()> {
    let command = serde_json::json!([{
        "name": "faucet",
        "description": "Get testnet RILL tokens sent to your address",
        "options": [{
            "name": "address",
            "description": "Your trill1... testnet address",
            "type": 3,
            "required": true
        }]
    }]);

    let client = reqwest::Client::new();
    let url = format!(
        "https://discord.com/api/v10/applications/{app_id}/commands"
    );

    let resp = client
        .put(&url)
        .header("Authorization", format!("Bot {token}"))
        .header("Content-Type", "application/json")
        .json(&command)
        .send()
        .await
        .context("Failed to send Discord command registration request")?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Discord API returned {status}: {body}");
    }

    Ok(())
}
