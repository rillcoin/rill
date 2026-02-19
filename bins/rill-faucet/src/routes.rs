//! Axum router and HTTP handlers.

use std::net::IpAddr;

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, warn};

use rill_core::constants::COIN;

use crate::discord;
use crate::send::{fetch_balance, rpc_client, send_rill};
use crate::AppState;

// Embed the web UI at compile time.
const INDEX_HTML: &str = include_str!("static/index.html");

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/", get(web_ui))
        .route("/api/faucet", post(api_faucet))
        .route("/api/status", get(api_status))
        .route("/discord/interactions", post(discord_interactions))
        .with_state(state)
        .layer(cors)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// Serve the embedded web UI.
async fn web_ui() -> Html<&'static str> {
    Html(INDEX_HTML)
}

#[derive(Deserialize)]
struct FaucetRequest {
    address: String,
}

/// `POST /api/faucet` — dispense RILL to the requested address.
async fn api_faucet(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<FaucetRequest>,
) -> impl IntoResponse {
    let ip = extract_ip(&headers);
    let address = req.address.trim().to_string();

    // Validate trill1... address prefix (testnet only).
    if !address.starts_with("trill1") {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Only testnet addresses (trill1...) are supported"})),
        );
    }

    // Rate limit check.
    {
        let limiter = state.rate_limiter.lock().await;
        if let Err((msg, _secs)) = limiter.check(&address, ip) {
            return (StatusCode::TOO_MANY_REQUESTS, Json(json!({"error": msg})));
        }
    }

    info!(%address, %ip, "Faucet request");

    let amount_rills = state.config.amount_rills;

    match send_rill(
        state.wallet.clone(),
        &state.wallet_path,
        state.wallet_password.as_slice(),
        &address,
        amount_rills,
        &state.config.rpc_endpoint,
    )
    .await
    {
        Ok(txid) => {
            info!(%txid, %address, "Faucet sent");
            let mut limiter = state.rate_limiter.lock().await;
            limiter.record(&address, ip);
            (
                StatusCode::OK,
                Json(json!({
                    "txid": txid,
                    "amount_rill": state.config.amount_rill(),
                    "address": address,
                })),
            )
        }
        Err(e) => {
            warn!(error = %e, %address, "Faucet send failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
        }
    }
}

/// `GET /api/status` — return node info and faucet wallet balance.
async fn api_status(State(state): State<AppState>) -> impl IntoResponse {
    let client = match rpc_client(&state.config.rpc_endpoint) {
        Ok(c) => c,
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"error": "Node unavailable"})),
            );
        }
    };

    use jsonrpsee::core::client::ClientT;
    use jsonrpsee::core::params::ArrayParams;

    let info: serde_json::Value = match client.request("getinfo", ArrayParams::new()).await {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({"error": format!("RPC error: {e}")})),
            );
        }
    };

    let height = info["blocks"].as_u64().unwrap_or(0);
    let network = info["network"].as_str().unwrap_or("testnet").to_string();

    // Collect wallet addresses (briefly hold the lock).
    let addresses: Vec<String> = {
        let mut wallet = state.wallet.lock().await;
        let count = wallet.address_count();
        (0..count)
            .map(|i| wallet.keychain_mut().address_at(i).encode())
            .collect()
    };

    let balance_rills = fetch_balance(&client, &addresses).await;

    (
        StatusCode::OK,
        Json(json!({
            "balance_rill": balance_rills as f64 / COIN as f64,
            "height": height,
            "network": network,
            "amount_per_claim_rill": state.config.amount_rill(),
            "cooldown_secs": state.config.cooldown_secs,
        })),
    )
}

/// `POST /discord/interactions` — handle Discord slash commands.
async fn discord_interactions(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    // Reject if Discord is not configured.
    let public_key = match &state.config.discord_public_key {
        Some(k) => k.clone(),
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Discord integration not configured"})),
            );
        }
    };

    // Verify Ed25519 signature (Discord will stop sending if this fails).
    let signature = headers
        .get("x-signature-ed25519")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let timestamp = headers
        .get("x-signature-timestamp")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !discord::verify_signature(&public_key, signature, timestamp, &body) {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Invalid signature"})));
    }

    let interaction: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid JSON"})));
        }
    };

    let interaction_type = interaction["type"].as_u64().unwrap_or(0);

    // Type 1 = PING (Discord health check).
    if interaction_type == 1 {
        return (StatusCode::OK, Json(json!({"type": 1})));
    }

    // Type 2 = APPLICATION_COMMAND.
    if interaction_type == 2 {
        let command_name = interaction["data"]["name"].as_str().unwrap_or("");
        if command_name == "faucet" {
            let address = interaction["data"]["options"]
                .as_array()
                .and_then(|opts| opts.iter().find(|o| o["name"] == "address"))
                .and_then(|o| o["value"].as_str())
                .unwrap_or("")
                .trim()
                .to_string();

            // Extract Discord user info for rate limiting (use user ID as key).
            let user_id = interaction["member"]["user"]["id"]
                .as_str()
                .or_else(|| interaction["user"]["id"].as_str())
                .unwrap_or(&address)
                .to_string();

            // Re-use address-based rate limiting keyed on Discord user ID.
            {
                let limiter = state.rate_limiter.lock().await;
                if let Err((msg, _)) = limiter.check(&user_id, IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED)) {
                    return (
                        StatusCode::OK,
                        Json(json!({
                            "type": 4,
                            "data": {
                                "content": format!("⏳ {msg}"),
                                "flags": 64  // ephemeral
                            }
                        })),
                    );
                }
            }

            if !address.starts_with("trill1") {
                return (
                    StatusCode::OK,
                    Json(json!({
                        "type": 4,
                        "data": {
                            "content": "❌ Please provide a valid testnet address (starts with `trill1`).",
                            "flags": 64
                        }
                    })),
                );
            }

            let amount_rills = state.config.amount_rills;
            let amount_rill = state.config.amount_rill();

            match send_rill(
                state.wallet.clone(),
                &state.wallet_path,
                state.wallet_password.as_slice(),
                &address,
                amount_rills,
                &state.config.rpc_endpoint,
            )
            .await
            {
                Ok(txid) => {
                    info!(txid = %txid, discord_user = %user_id, %address, "Discord faucet sent");
                    let mut limiter = state.rate_limiter.lock().await;
                    limiter.record(&user_id, IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED));
                    (
                        StatusCode::OK,
                        Json(json!({
                            "type": 4,
                            "data": {
                                "content": format!(
                                    "✅ Sent **{amount_rill} RILL** to `{address}`\n🔗 TxID: `{txid}`"
                                ),
                                "flags": 64
                            }
                        })),
                    )
                }
                Err(e) => {
                    warn!(error = %e, discord_user = %user_id, "Discord faucet send failed");
                    (
                        StatusCode::OK,
                        Json(json!({
                            "type": 4,
                            "data": {
                                "content": format!("❌ Failed to send: {e}"),
                                "flags": 64
                            }
                        })),
                    )
                }
            }
        } else {
            (StatusCode::OK, Json(json!({"type": 4, "data": {"content": "Unknown command.", "flags": 64}})))
        }
    } else {
        (StatusCode::BAD_REQUEST, Json(json!({"error": "Unsupported interaction type"})))
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract the client IP from `X-Forwarded-For` or `X-Real-IP` headers.
fn extract_ip(headers: &HeaderMap) -> IpAddr {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .and_then(|s| s.trim().parse().ok())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.trim().parse().ok())
        })
        .unwrap_or(IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED))
}
