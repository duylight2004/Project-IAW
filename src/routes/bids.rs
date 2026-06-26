// src/routes/bids.rs — Act 3 (TOCTOU double-settle)
//
// settle: READ trạng thái -> WINDOW (sleep mô phỏng xử lý chậm) -> WRITE.
// Không atomic, không BEGIN IMMEDIATE, không lock. Hai request song song
// cùng đọc status='open' trong window -> cùng ghi settlement -> COUNT > 1.
use axum::{extract::State, response::IntoResponse, Json};
use serde::Deserialize;
use serde_json::json;
use sqlx::SqlitePool;
use std::time::Duration;

use crate::models::Bid;

// Flag 3 — KHÔNG nằm trong DB; server trả khi phát hiện double-settle.
const FLAG3: &str = "FLAG{t0ct0u_d0ubl3_s3ttl3m3nt_r4c3}";
const WINDOW_MS: u64 = 250;

#[derive(Deserialize)]
pub struct SettleReq {
    pub bid_id: i64,
    pub who: String,
}

pub async fn settle(State(pool): State<SqlitePool>, Json(req): Json<SettleReq>) -> impl IntoResponse {
    // 1) READ
    let bid: Option<Bid> = sqlx::query_as::<_, Bid>("SELECT * FROM bids WHERE id = ?")
        .bind(req.bid_id)
        .fetch_optional(&pool)
        .await
        .unwrap_or(None);
    let Some(bid) = bid else {
        return (
            axum::http::StatusCode::NOT_FOUND,
            Json(json!({ "error": "bid not found" })),
        )
            .into_response();
    };
    if bid.status != "open" {
        return (
            axum::http::StatusCode::CONFLICT,
            Json(json!({ "error": "auction closed" })),
        )
            .into_response();
    }

    // 2) WINDOW — nguồn race (không atomic giữa read và write)
    tokio::time::sleep(Duration::from_millis(WINDOW_MS)).await;

    // 3) WRITE + audit
    sqlx::query("UPDATE bids SET status = 'settled', winner = ? WHERE id = ?")
        .bind(&req.who)
        .bind(req.bid_id)
        .execute(&pool)
        .await
        .ok();
    sqlx::query("INSERT INTO settlements (bid_id, who) VALUES (?, ?)")
        .bind(req.bid_id)
        .bind(&req.who)
        .execute(&pool)
        .await
        .ok();

    // 4) detect double-settle -> FLAG 3
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM settlements WHERE bid_id = ?")
        .bind(req.bid_id)
        .fetch_one(&pool)
        .await
        .unwrap_or(0);

    if n > 1 {
        Json(json!({
            "status": "double_settle_detected",
            "settlements": n,
            "flag": FLAG3,
        }))
        .into_response()
    } else {
        Json(json!({
            "status": "settled",
            "processing_window_ms": WINDOW_MS,
        }))
        .into_response()
    }
}
