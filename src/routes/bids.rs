// src/routes/bids.rs — Act 3 (TOCTOU double-settle) + nonce dùng-một-lần
//
// settle: READ trạng thái -> WINDOW (sleep mô phỏng xử lý chậm) -> WRITE.
// Không atomic, không BEGIN IMMEDIATE, không lock. Hai request song song
// cùng đọc status='open' trong window -> cùng ghi settlement -> COUNT > 1.
//
// CHỐNG REPLAY: mỗi settle cần một `ticket` ngẫu nhiên dùng-một-lần (lấy qua
// POST /bids/ticket, cần BIDDER_PASS từ Act 2). Tiêu thụ ticket nguyên tử bằng
// UPDATE ... WHERE used=0 -> bắn lại cùng một request không ăn lần hai. Muốn
// double-settle phải xin 2 ticket khác nhau rồi đua trong window.
use axum::{extract::State, response::IntoResponse, Json};
use serde::Deserialize;
use serde_json::json;
use sqlx::SqlitePool;
use std::time::Duration;

use argon2::password_hash::{rand_core::OsRng, SaltString};

use crate::models::Bid;
use crate::routes::books::BIDDER_PASS;

// Flag 3 — KHÔNG nằm trong DB; server trả khi phát hiện double-settle.
const FLAG3: &str = "IAW{t0ct0u_d0ubl3_s3ttl3m3nt_r4c3}";
const WINDOW_MS: u64 = 250;

// ─────────────────────────── POST /bids/ticket ───────────────────────────

#[derive(Deserialize)]
pub struct TicketReq {
    pub bidder_pass: String,
    /// phiên muốn settle (mặc định bid 1 nếu không truyền)
    pub bid_id: Option<i64>,
}

/// Cấp một nonce dùng-một-lần cho settle. Cần BIDDER_PASS (token ẩn từ Act 2).
/// Token sinh ngẫu nhiên (OsRng) -> không đoán/precompute được, buộc fetch động.
pub async fn ticket(
    State(pool): State<SqlitePool>,
    headers: axum::http::HeaderMap,
    Json(req): Json<TicketReq>,
) -> impl IntoResponse {
    if !crate::routes::auth::is_logged_in(&headers) {
        return (
            axum::http::StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "unauthorized", "message": "Please log in" })),
        )
            .into_response();
    }

    // CHUỖI Act 2 -> Act 3: chỉ "verified bidder" (có BIDDER_PASS) mới xin được ticket.
    if req.bidder_pass != BIDDER_PASS {
        return (
            axum::http::StatusCode::FORBIDDEN,
            Json(json!({ "error": "not_verified_bidder" })),
        )
            .into_response();
    }

    let bid_id = req.bid_id.unwrap_or(1);
    let token = SaltString::generate(&mut OsRng).as_str().to_owned();

    let inserted = sqlx::query("INSERT INTO tickets (token, bid_id, used) VALUES (?, ?, 0)")
        .bind(&token)
        .bind(bid_id)
        .execute(&pool)
        .await;

    if inserted.is_err() {
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "ticket_issue_failed" })),
        )
            .into_response();
    }

    Json(json!({ "ticket": token, "bid_id": bid_id })).into_response()
}

// ─────────────────────────── POST /bids/settle ───────────────────────────

#[derive(Deserialize)]
pub struct SettleReq {
    pub bid_id: i64,
    pub who: String,
    /// nonce dùng-một-lần lấy từ /bids/ticket
    pub ticket: String,
    /// token ẩn từ Act 2
    pub bidder_pass: String,
}

pub async fn settle(
    State(pool): State<SqlitePool>,
    headers: axum::http::HeaderMap,
    Json(req): Json<SettleReq>,
) -> impl IntoResponse {
    if !crate::routes::auth::is_logged_in(&headers) {
        return (
            axum::http::StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "unauthorized", "message": "Please log in to join the auction" })),
        )
            .into_response();
    }

    // CHUỖI Act 2 -> Act 3: cần BIDDER_PASS.
    if req.bidder_pass != BIDDER_PASS {
        return (
            axum::http::StatusCode::FORBIDDEN,
            Json(json!({ "error": "not_verified_bidder" })),
        )
            .into_response();
    }

    // CHỐNG REPLAY: tiêu thụ ticket nguyên tử. rows_affected==0 nghĩa là ticket
    // không tồn tại hoặc đã dùng -> chặn bắn lại cùng một request.
    let consumed = sqlx::query("UPDATE tickets SET used = 1 WHERE token = ? AND used = 0")
        .bind(&req.ticket)
        .execute(&pool)
        .await;
    let ok_ticket = consumed.map(|r| r.rows_affected() == 1).unwrap_or(false);
    if !ok_ticket {
        return (
            axum::http::StatusCode::CONFLICT,
            Json(json!({ "error": "ticket_invalid", "message": "Ticket invalid or already used" })),
        )
            .into_response();
    }

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
