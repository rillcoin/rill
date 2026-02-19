use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Json, Response},
    routing::get,
    Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use tower_http::cors::{Any, CorsLayer};

const COIN: u64 = 100_000_000;

// ── Error helper ─────────────────────────────────────────────────────────────

struct ApiError(anyhow::Error);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = json!({ "error": self.0.to_string() });
        (StatusCode::INTERNAL_SERVER_ERROR, Json(body)).into_response()
    }
}

impl<E: Into<anyhow::Error>> From<E> for ApiError {
    fn from(e: E) -> Self { ApiError(e.into()) }
}

type ApiResult<T> = Result<Json<T>, ApiError>;

// ── Router ───────────────────────────────────────────────────────────────────

pub fn router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/", get(web_ui))
        .route("/api/stats", get(stats))
        .route("/api/blocks", get(recent_blocks))
        .route("/api/block/{id}", get(block_detail))
        .route("/api/tx/{txid}", get(tx_detail))
        .route("/api/address/{addr}", get(address_detail))
        .route("/api/search", get(search))
        .layer(cors)
        .with_state(state)
}

const INDEX_HTML: &str = include_str!("static/index.html");

async fn web_ui() -> Html<&'static str> {
    Html(INDEX_HTML)
}

// ── /api/stats ────────────────────────────────────────────────────────────────

async fn stats(State(s): State<AppState>) -> ApiResult<Value> {
    let (chain, mempool) = tokio::join!(
        s.rpc.get_blockchain_info(),
        s.rpc.get_mempool_info(),
    );
    let chain = chain?;
    let mempool = mempool.unwrap_or_else(|_| json!({"size":0,"bytes":0,"total_fee":0}));

    Ok(Json(json!({
        "height":             chain["height"],
        "best_hash":          chain["best_block_hash"],
        "circulating_supply": rills_to_rill(chain["circulating_supply"].as_u64().unwrap_or(0)),
        "decay_pool":         rills_to_rill(chain["decay_pool_balance"].as_u64().unwrap_or(0)),
        "utxo_count":         chain["utxo_count"],
        "ibd":                chain["initial_block_download"],
        "peer_count":         chain["peer_count"],
        "mempool_size":       mempool["size"],
        "mempool_bytes":      mempool["bytes"],
    })))
}

// ── /api/blocks?limit=N ───────────────────────────────────────────────────────

#[derive(Deserialize)]
struct BlocksQuery {
    limit: Option<u64>,
    offset: Option<u64>,
}

async fn recent_blocks(
    State(s): State<AppState>,
    Query(q): Query<BlocksQuery>,
) -> ApiResult<Value> {
    let limit = q.limit.unwrap_or(20).min(50);
    let tip = s.rpc.get_block_count().await?;
    let offset = q.offset.unwrap_or(0);

    let start = tip.saturating_sub(offset);
    let count = limit.min(start + 1);

    let mut blocks = Vec::with_capacity(count as usize);
    for i in 0..count {
        let height = start.saturating_sub(i);
        if let Ok(hash) = s.rpc.get_block_hash(height).await {
            if let Ok(block) = s.rpc.get_block(&hash).await {
                blocks.push(json!({
                    "hash":      block["hash"],
                    "height":    block["height"],
                    "timestamp": block["timestamp"],
                    "tx_count":  block["tx_count"],
                    "prev_hash": block["prev_hash"],
                }));
            }
        }
    }

    Ok(Json(json!({
        "tip":    tip,
        "blocks": blocks,
    })))
}

// ── /api/block/:id ────────────────────────────────────────────────────────────

async fn block_detail(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Value> {
    // id is either a height (numeric) or a hash (64-char hex)
    let hash = if id.len() == 64 && id.chars().all(|c| c.is_ascii_hexdigit()) {
        id
    } else if let Ok(height) = id.parse::<u64>() {
        s.rpc.get_block_hash(height).await?
    } else {
        return Err(ApiError(anyhow::anyhow!("invalid block id: {}", id)));
    };

    let block = s.rpc.get_block(&hash).await?;
    Ok(Json(block))
}

// ── /api/tx/:txid ─────────────────────────────────────────────────────────────

async fn tx_detail(
    State(s): State<AppState>,
    Path(txid): Path<String>,
) -> ApiResult<Value> {
    let tx = s.rpc.get_transaction(&txid).await?;
    Ok(Json(tx))
}

// ── /api/address/:addr ────────────────────────────────────────────────────────

async fn address_detail(
    State(s): State<AppState>,
    Path(addr): Path<String>,
) -> ApiResult<Value> {
    let utxos = s.rpc.get_utxos_by_address(&addr).await?;

    let balance_rills: u64 = utxos
        .iter()
        .filter_map(|u| u["value"].as_u64())
        .sum();

    let utxo_list: Vec<Value> = utxos
        .iter()
        .map(|u| json!({
            "txid":         u["txid"],
            "index":        u["index"],
            "value_rill":   rills_to_rill(u["value"].as_u64().unwrap_or(0)),
            "value_rills":  u["value"],
            "block_height": u["block_height"],
            "is_coinbase":  u["is_coinbase"],
            "cluster_id":   u["cluster_id"],
        }))
        .collect();

    Ok(Json(json!({
        "address":      addr,
        "balance_rill": rills_to_rill(balance_rills),
        "utxo_count":   utxo_list.len(),
        "utxos":        utxo_list,
    })))
}

// ── /api/search?q=... ─────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
}

async fn search(
    State(s): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> ApiResult<Value> {
    let q = q.q.trim().to_owned();

    // Address
    if q.starts_with("trill1") || q.starts_with("rill1") {
        return Ok(Json(json!({ "type": "address", "value": q })));
    }

    // Height
    if let Ok(height) = q.parse::<u64>() {
        let tip = s.rpc.get_block_count().await?;
        if height <= tip {
            return Ok(Json(json!({ "type": "block", "value": q })));
        }
    }

    // 64-char hex — try block then tx
    if q.len() == 64 && q.chars().all(|c| c.is_ascii_hexdigit()) {
        if s.rpc.get_block(&q).await.is_ok() {
            return Ok(Json(json!({ "type": "block", "value": q })));
        }
        if s.rpc.get_transaction(&q).await.is_ok() {
            return Ok(Json(json!({ "type": "tx", "value": q })));
        }
    }

    Err(ApiError(anyhow::anyhow!("not found: {}", q)))
}

// ── Utils ────────────────────────────────────────────────────────────────────

fn rills_to_rill(rills: u64) -> f64 {
    rills as f64 / COIN as f64
}
