// src/routes/auth.rs — RABBIT HOLE (Act 1)
//
// /login dùng prepared statement chuẩn: username/password đều được BIND,
// không nối chuỗi -> mọi payload SQLi tại đây đều fail. Cố ý vô hại để tốn
// thời gian người chơi. Con đường thật là GET /search (xem books.rs).
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{extract::State, response::IntoResponse, Form, Json};
use serde::Deserialize;
use serde_json::json;
use sqlx::SqlitePool;

use crate::models::User;

#[derive(Deserialize)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

pub async fn login(State(pool): State<SqlitePool>, Form(form): Form<LoginForm>) -> impl IntoResponse {
    // Prepared statement: username được bind -> không injectable.
    let user: Option<User> = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = ?")
        .bind(&form.username)
        .fetch_optional(&pool)
        .await
        .unwrap_or(None);

    let ok = match &user {
        Some(u) => PasswordHash::new(&u.password)
            .map(|ph| {
                Argon2::default()
                    .verify_password(form.password.as_bytes(), &ph)
                    .is_ok()
            })
            .unwrap_or(false),
        None => false,
    };

    if ok {
        let cookie = format!("antiqua_session={}; Path=/; SameSite=Lax", form.username);
        (
            [(axum::http::header::SET_COOKIE, cookie)],
            Json(json!({ "status": "ok", "user": form.username })),
        )
            .into_response()
    } else {
        (
            axum::http::StatusCode::UNAUTHORIZED,
            Json(json!({ "status": "error", "message": "invalid credentials" })),
        )
            .into_response()
    }
}

pub fn is_logged_in(headers: &axum::http::HeaderMap) -> bool {
    headers
        .get(axum::http::header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.contains("antiqua_session="))
        .unwrap_or(false)
}
