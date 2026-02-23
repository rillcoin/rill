//! Axum router and HTTP handlers.

use std::net::IpAddr;

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse};
use axum::routing::{get, post};
use axum::{Json, Router};
use axum::extract::Query;
use serde::Deserialize;
use serde_json::json;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, warn};

use rill_core::address::Network;
use rill_core::constants::COIN;
use rill_wallet::{seed_to_mnemonic, KeyChain, Seed};

use crate::discord;
use crate::send::{
    create_contract_from_mnemonic, fetch_balance, fetch_balance_for_address,
    fulfil_contract_from_mnemonic, register_agent_from_mnemonic, rpc_client,
    send_from_mnemonic, send_rill, submit_review_from_mnemonic, vouch_from_mnemonic,
};
use crate::AppState;

// Embed the web UI at compile time.
const INDEX_HTML: &str = include_str!("static/index.html");
const CREATE_WALLET_HTML: &str = include_str!("static/create_wallet.html");

// Embed discovery files at compile time.
const WELL_KNOWN_AGENTS: &str = include_str!("static/well-known-rill-agents.json");
const AI_PLUGIN: &str = include_str!("static/ai-plugin.json");
const OPENAPI_SPEC: &str = include_str!("static/openapi.json");

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
        .route("/create-wallet", get(create_wallet_ui))
        .route("/api/faucet", post(api_faucet))
        .route("/api/status", get(api_status))
        .route("/api/wallet/new", get(api_create_wallet))
        .route("/api/wallet/balance", get(api_wallet_balance))
        .route("/api/wallet/send", post(api_wallet_send))
        .route("/api/wallet/derive", post(api_wallet_derive))
        .route("/discord/interactions", post(discord_interactions))
        // -- Agent endpoints --
        .route("/api/agent/register", post(api_agent_register))
        .route("/api/agent/profile", get(api_agent_profile))
        .route("/api/agent/directory", get(api_agent_directory))
        .route("/api/agent/vouch", post(api_agent_vouch))
        .route("/api/agent/contract/create", post(api_agent_contract_create))
        .route("/api/agent/contract/fulfil", post(api_agent_contract_fulfil))
        .route("/api/agent/review", post(api_agent_review))
        // -- Discovery endpoints --
        .route("/.well-known/rill-agents.json", get(well_known_agents))
        .route("/.well-known/ai-plugin.json", get(ai_plugin))
        .route("/api/openapi.json", get(openapi_spec))
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

/// Serve the wallet creation page.
async fn create_wallet_ui() -> Html<&'static str> {
    Html(CREATE_WALLET_HTML)
}

/// `GET /api/wallet/new` — generate a fresh testnet wallet.
///
/// Generates a random seed, derives a BIP-39 mnemonic and the first testnet
/// address. The seed is **never stored** on the server — the caller must save
/// the mnemonic to restore the wallet later.
async fn api_create_wallet() -> impl IntoResponse {
    let seed = Seed::generate();
    let mnemonic = seed_to_mnemonic(&seed);
    let mut keychain = KeyChain::new(seed, Network::Testnet);
    let address = keychain.address_at(0).encode();

    Json(json!({
        "mnemonic": mnemonic,
        "address": address,
    }))
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
// Web Wallet API
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct BalanceQuery {
    address: String,
}

/// `GET /api/wallet/balance?address=trill1...` — look up balance for any address.
async fn api_wallet_balance(
    State(state): State<AppState>,
    Query(query): Query<BalanceQuery>,
) -> impl IntoResponse {
    let address = query.address.trim().to_string();

    if !address.starts_with("trill1") {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Only testnet addresses (trill1...) are supported"})),
        );
    }

    let client = match rpc_client(&state.config.rpc_endpoint) {
        Ok(c) => c,
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"error": "Node unavailable"})),
            );
        }
    };

    match fetch_balance_for_address(&client, &address).await {
        Ok((balance_rills, utxo_count)) => (
            StatusCode::OK,
            Json(json!({
                "address": address,
                "balance_rill": balance_rills as f64 / COIN as f64,
                "balance_rills": balance_rills,
                "utxo_count": utxo_count,
            })),
        ),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({"error": format!("Failed to fetch balance: {e}")})),
        ),
    }
}

#[derive(Deserialize)]
struct WalletSendRequest {
    mnemonic: String,
    to: String,
    amount_rill: f64,
}

/// `POST /api/wallet/send` — send RILL from a mnemonic-derived wallet.
async fn api_wallet_send(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<WalletSendRequest>,
) -> impl IntoResponse {
    let ip = extract_ip(&headers);

    // Rate limit by IP (1 send per 30 seconds) — separate from faucet cooldown.
    {
        let limiter = state.wallet_rate_limiter.lock().await;
        let key = format!("wallet_send:{ip}");
        if let Err((msg, _)) = limiter.check(&key, ip) {
            return (StatusCode::TOO_MANY_REQUESTS, Json(json!({"error": msg})));
        }
    }

    let to = req.to.trim().to_string();
    if !to.starts_with("trill1") {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Only testnet addresses (trill1...) are supported"})),
        );
    }

    if req.amount_rill <= 0.0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Amount must be greater than zero"})),
        );
    }

    let amount_rills = (req.amount_rill * COIN as f64) as u64;

    match send_from_mnemonic(&req.mnemonic, &to, amount_rills, &state.config.rpc_endpoint).await {
        Ok((txid, fee)) => {
            info!(%txid, %to, amount_rill = req.amount_rill, "Wallet send succeeded");
            let mut limiter = state.wallet_rate_limiter.lock().await;
            let key = format!("wallet_send:{ip}");
            limiter.record(&key, ip);
            (
                StatusCode::OK,
                Json(json!({
                    "txid": txid,
                    "amount_rill": req.amount_rill,
                    "fee_rill": fee as f64 / COIN as f64,
                })),
            )
        }
        Err(e) => {
            warn!(error = %e, "Wallet send failed");
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": e.to_string()})),
            )
        }
    }
}

#[derive(Deserialize)]
struct DeriveRequest {
    mnemonic: String,
}

/// `POST /api/wallet/derive` — derive address from mnemonic (for restore flow).
async fn api_wallet_derive(Json(req): Json<DeriveRequest>) -> impl IntoResponse {
    let mnemonic = req.mnemonic.trim();

    match rill_wallet::mnemonic_to_seed(mnemonic) {
        Ok(seed) => {
            let mut keychain = KeyChain::new(seed, Network::Testnet);
            let address = keychain.address_at(0).encode();
            (StatusCode::OK, Json(json!({"address": address})))
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("Invalid mnemonic: {e}")})),
        ),
    }
}

// ---------------------------------------------------------------------------
// Discovery endpoints
// ---------------------------------------------------------------------------

/// `GET /.well-known/rill-agents.json` — machine-readable agent discovery.
async fn well_known_agents() -> impl IntoResponse {
    (
        [("content-type", "application/json")],
        WELL_KNOWN_AGENTS,
    )
}

/// `GET /.well-known/ai-plugin.json` — ChatGPT plugin manifest.
async fn ai_plugin() -> impl IntoResponse {
    (
        [("content-type", "application/json")],
        AI_PLUGIN,
    )
}

/// `GET /api/openapi.json` — OpenAPI 3.0 specification.
async fn openapi_spec() -> impl IntoResponse {
    (
        [("content-type", "application/json")],
        OPENAPI_SPEC,
    )
}

// ---------------------------------------------------------------------------
// Agent API
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct AgentRegisterRequest {
    mnemonic: String,
}

/// `POST /api/agent/register` — register a wallet as an agent.
async fn api_agent_register(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<AgentRegisterRequest>,
) -> impl IntoResponse {
    let ip = extract_ip(&headers);

    // Rate limit by IP.
    {
        let limiter = state.wallet_rate_limiter.lock().await;
        let key = format!("agent_register:{ip}");
        if let Err((msg, _)) = limiter.check(&key, ip) {
            return (StatusCode::TOO_MANY_REQUESTS, Json(json!({"error": msg})));
        }
    }

    match register_agent_from_mnemonic(&req.mnemonic, &state.config.rpc_endpoint).await {
        Ok(txid) => {
            info!(%txid, "Agent registration submitted");
            let mut limiter = state.wallet_rate_limiter.lock().await;
            let key = format!("agent_register:{ip}");
            limiter.record(&key, ip);
            (StatusCode::OK, Json(json!({"txid": txid})))
        }
        Err(e) => {
            warn!(error = %e, "Agent registration failed");
            (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()})))
        }
    }
}

#[derive(Deserialize)]
struct AgentProfileQuery {
    address: String,
}

/// `GET /api/agent/profile?address=trill1...` — get agent conduct profile.
async fn api_agent_profile(
    State(state): State<AppState>,
    Query(query): Query<AgentProfileQuery>,
) -> impl IntoResponse {
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

    let mut params = ArrayParams::new();
    params.insert(query.address.clone()).unwrap();

    match client
        .request::<serde_json::Value, _>("getAgentConductProfile", params)
        .await
    {
        Ok(profile) => (StatusCode::OK, Json(profile)),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({"error": format!("RPC error: {e}")})),
        ),
    }
}

#[derive(Deserialize)]
struct AgentDirectoryQuery {
    #[serde(default)]
    offset: usize,
    #[serde(default = "default_directory_limit")]
    limit: usize,
}

fn default_directory_limit() -> usize {
    20
}

/// `GET /api/agent/directory?offset=0&limit=20` — paginated agent directory.
async fn api_agent_directory(
    State(state): State<AppState>,
    Query(query): Query<AgentDirectoryQuery>,
) -> impl IntoResponse {
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

    let mut params = ArrayParams::new();
    params.insert(query.offset).unwrap();
    params.insert(query.limit.min(100)).unwrap();

    match client
        .request::<serde_json::Value, _>("listAgentWallets", params)
        .await
    {
        Ok(directory) => (StatusCode::OK, Json(directory)),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({"error": format!("RPC error: {e}")})),
        ),
    }
}

#[derive(Deserialize)]
struct VouchRequest {
    mnemonic: String,
    target_address: String,
}

/// `POST /api/agent/vouch` — vouch for another agent.
async fn api_agent_vouch(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<VouchRequest>,
) -> impl IntoResponse {
    let ip = extract_ip(&headers);
    {
        let limiter = state.wallet_rate_limiter.lock().await;
        let key = format!("agent_vouch:{ip}");
        if let Err((msg, _)) = limiter.check(&key, ip) {
            return (StatusCode::TOO_MANY_REQUESTS, Json(json!({"error": msg})));
        }
    }

    match vouch_from_mnemonic(&req.mnemonic, &req.target_address, &state.config.rpc_endpoint).await
    {
        Ok(txid) => {
            info!(%txid, "Vouch submitted");
            let mut limiter = state.wallet_rate_limiter.lock().await;
            let key = format!("agent_vouch:{ip}");
            limiter.record(&key, ip);
            (StatusCode::OK, Json(json!({"txid": txid})))
        }
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()}))),
    }
}

#[derive(Deserialize)]
struct ContractCreateRequest {
    mnemonic: String,
    counterparty: String,
    value_rill: f64,
}

/// `POST /api/agent/contract/create` — create an agent contract.
async fn api_agent_contract_create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<ContractCreateRequest>,
) -> impl IntoResponse {
    let ip = extract_ip(&headers);
    {
        let limiter = state.wallet_rate_limiter.lock().await;
        let key = format!("agent_contract:{ip}");
        if let Err((msg, _)) = limiter.check(&key, ip) {
            return (StatusCode::TOO_MANY_REQUESTS, Json(json!({"error": msg})));
        }
    }

    if req.value_rill <= 0.0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Contract value must be greater than zero"})),
        );
    }

    let value_rills = (req.value_rill * COIN as f64) as u64;

    match create_contract_from_mnemonic(
        &req.mnemonic,
        &req.counterparty,
        value_rills,
        &state.config.rpc_endpoint,
    )
    .await
    {
        Ok(txid) => {
            info!(%txid, "Contract created");
            let mut limiter = state.wallet_rate_limiter.lock().await;
            let key = format!("agent_contract:{ip}");
            limiter.record(&key, ip);
            (StatusCode::OK, Json(json!({"txid": txid})))
        }
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()}))),
    }
}

#[derive(Deserialize)]
struct ContractFulfilRequest {
    mnemonic: String,
    contract_id: String,
}

/// `POST /api/agent/contract/fulfil` — fulfil an agent contract.
async fn api_agent_contract_fulfil(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<ContractFulfilRequest>,
) -> impl IntoResponse {
    let ip = extract_ip(&headers);
    {
        let limiter = state.wallet_rate_limiter.lock().await;
        let key = format!("agent_fulfil:{ip}");
        if let Err((msg, _)) = limiter.check(&key, ip) {
            return (StatusCode::TOO_MANY_REQUESTS, Json(json!({"error": msg})));
        }
    }

    match fulfil_contract_from_mnemonic(
        &req.mnemonic,
        &req.contract_id,
        &state.config.rpc_endpoint,
    )
    .await
    {
        Ok(txid) => {
            info!(%txid, "Contract fulfilled");
            let mut limiter = state.wallet_rate_limiter.lock().await;
            let key = format!("agent_fulfil:{ip}");
            limiter.record(&key, ip);
            (StatusCode::OK, Json(json!({"txid": txid})))
        }
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()}))),
    }
}

#[derive(Deserialize)]
struct ReviewRequest {
    mnemonic: String,
    subject_address: String,
    score: u64,
    contract_id: String,
}

/// `POST /api/agent/review` — submit a peer review.
async fn api_agent_review(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<ReviewRequest>,
) -> impl IntoResponse {
    let ip = extract_ip(&headers);
    {
        let limiter = state.wallet_rate_limiter.lock().await;
        let key = format!("agent_review:{ip}");
        if let Err((msg, _)) = limiter.check(&key, ip) {
            return (StatusCode::TOO_MANY_REQUESTS, Json(json!({"error": msg})));
        }
    }

    match submit_review_from_mnemonic(
        &req.mnemonic,
        &req.subject_address,
        req.score,
        &req.contract_id,
        &state.config.rpc_endpoint,
    )
    .await
    {
        Ok(txid) => {
            info!(%txid, "Review submitted");
            let mut limiter = state.wallet_rate_limiter.lock().await;
            let key = format!("agent_review:{ip}");
            limiter.record(&key, ip);
            (StatusCode::OK, Json(json!({"txid": txid})))
        }
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()}))),
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
