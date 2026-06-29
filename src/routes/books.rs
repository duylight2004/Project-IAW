// src/routes/books.rs — Act 1 (SQLi qua format!) + Act 2 (integer overflow)
use axum::{extract::State, extract::Query, response::IntoResponse, Json};
use serde::Deserialize;
use serde_json::json;
use sqlx::{Row, SqlitePool};

use crate::models::{Book, User};
use crate::pricing::total_price;

// Flag 2 — KHÔNG nằm trong DB. Chỉ render trong handler khi mua thành công.
const FLAG2: &str = "IAW{r3l34s3_0v3rfl0w_fr33_r3str1ct3d_b00k}";

// ─────────────────────────── Act 1: /search ───────────────────────────

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

/// LỖ HỔNG CỐ Ý: nối thẳng input vào câu SQL bằng format!, không bind.
/// SELECT trả 3 cột (id:int, title:text, author:text) -> UNION khớp 3 cột.
pub async fn search(
    State(pool): State<SqlitePool>,
    Query(p): Query<SearchQuery>,
) -> impl IntoResponse {
    let sql = format!(
        "SELECT id, title, author FROM books WHERE title LIKE '%{}%'",
        p.q
    );

    match sqlx::query(&sql).fetch_all(&pool).await {
        Ok(rows) => {
            let mut out = Vec::with_capacity(rows.len());
            for r in rows {
                let id: i64 = r.try_get(0).unwrap_or_default();
                let c1: Option<String> = r.try_get(1).ok().flatten();
                let c2: Option<String> = r.try_get(2).ok().flatten();
                out.push(json!({
                    "id": id,
                    "title": c1.unwrap_or_default(),
                    "author": c2.unwrap_or_default(),
                }));
            }
            Json(json!({ "results": out })).into_response()
        }
        // legacy search engine: lộ lỗi SQL ra ngoài để người chơi enumerate cột.
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

// ─────────────────────────── /catalog ───────────────────────────

pub async fn catalog(State(pool): State<SqlitePool>) -> impl IntoResponse {
    let books: Vec<Book> = sqlx::query_as::<_, Book>("SELECT * FROM books ORDER BY id")
        .fetch_all(&pool)
        .await
        .unwrap_or_default();
    Json(json!({ "books": books }))
}

// ─────────────────────────── Act 2: /buy ───────────────────────────

#[derive(Deserialize)]
pub struct BuyForm {
    /// mặc định = sách restricted nếu không truyền
    pub book_id: Option<i64>,
    /// input runtime -> không const-fold; phép nhân tràn ở release build
    pub quantity: u32,
}

pub async fn buy(
    State(pool): State<SqlitePool>,
    headers: axum::http::HeaderMap,
    Json(form): Json<BuyForm>,
) -> impl IntoResponse {
    if !crate::routes::auth::is_logged_in(&headers) {
        return (
            axum::http::StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "unauthorized", "message": "Vui lòng đăng nhập để mua sách" })),
        )
            .into_response();
    }

    // Chặn quantity=0: nếu không, cost = price*0 = 0 <= coins -> mua miễn phí,
    // bypass cả Act 2 (integer overflow). Phải mua ít nhất 1 cuốn.
    if form.quantity == 0 {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(json!({ "error": "invalid_quantity", "message": "Số lượng phải >= 1" })),
        )
            .into_response();
    }

    // người mua mặc định = seeker
    let user: Option<User> = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = 'seeker'")
        .fetch_optional(&pool)
        .await
        .unwrap_or(None);
    let Some(user) = user else {
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "no buyer" })),
        )
            .into_response();
    };

    // sách: theo book_id, hoặc sách restricted đầu tiên
    let book: Option<Book> = match form.book_id {
        Some(id) => sqlx::query_as::<_, Book>("SELECT * FROM books WHERE id = ?")
            .bind(id)
            .fetch_optional(&pool)
            .await
            .unwrap_or(None),
        None => sqlx::query_as::<_, Book>("SELECT * FROM books WHERE restricted = 1 LIMIT 1")
            .fetch_optional(&pool)
            .await
            .unwrap_or(None),
    };
    let Some(book) = book else {
        return (
            axum::http::StatusCode::NOT_FOUND,
            Json(json!({ "error": "book not found" })),
        )
            .into_response();
    };

    // LỖ HỔNG: total_price không kiểm tràn; quantity lớn -> wraps quanh 2^32.
    let cost = total_price(book.price as u32, form.quantity);

    if (cost as i64) <= user.coins {
        // cấp quyền đọc bất kể quantity, chỉ cần cost <= coins
        sqlx::query("UPDATE users SET coins = coins - ? WHERE id = ?")
            .bind(cost as i64)
            .bind(user.id)
            .execute(&pool)
            .await
            .ok();
        sqlx::query("INSERT INTO purchases (user_id, book_id) VALUES (?, ?)")
            .bind(user.id)
            .bind(book.id)
            .execute(&pool)
            .await
            .ok();

        let body = if book.restricted == 1 {
            // nội dung thật của sách restricted — chứa FLAG 2
            format!(
                "Bạn đã sở hữu '{}'. Nội dung khoá đã mở: {}",
                book.title, FLAG2
            )
        } else {
            format!("Bạn đã mua '{}'.", book.title)
        };

        Json(json!({
            "status": "purchased",
            "total_price": cost,
            "quantity": form.quantity,
            "body": body,
        }))
        .into_response()
    } else {
        // mua hụt: lộ giá đã tính để người chơi quan sát overflow
        Json(json!({
            "status": "insufficient_funds",
            "total_price": cost,
            "balance": user.coins,
        }))
        .into_response()
    }
}
