mod db;
mod models;
mod pricing;
mod routes;

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use serde_json::json;
use sqlx::SqlitePool;
use tower_http::services::ServeDir;

// Token tĩnh cho endpoint reset (infra/test, không phải mục tiêu CTF).
const RESET_TOKEN: &str = "antiqua-reset-7f3a";

#[tokio::main]
async fn main() {
    let db_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:///tmp/db/antiqua.db".to_string());

    let pool = db::init_pool(&db_url).await;
    db::init_db(&pool).await;

    let app = Router::new()
        .route("/", get(index))
        .route("/login", get(login_page).post(routes::auth::login))
        .route("/search", get(routes::books::search))
        .route("/catalog", get(routes::books::catalog))
        .route("/buy", post(routes::books::buy))
        .route("/bids/settle", post(routes::bids::settle))
        .route("/admin/reset", post(admin_reset))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(pool);

    let addr = "0.0.0.0:8080";
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("bind 0.0.0.0:8080");
    println!("Antiqua Library listening on http://{addr}");
    axum::serve(listener, app).await.expect("serve");
}

// Trang chủ — phục vụ HTML nguồn kèm hint (xem comment trong index.html).
async fn index() -> Html<&'static str> {
    Html(include_str!("../templates/index.html"))
}

// GET /login — form đăng nhập (rabbit hole). POST /login xử lý ở routes::auth.
async fn login_page() -> Html<&'static str> {
    Html(include_str!("../templates/login.html"))
}

// POST /admin/reset — reset Act 2/3, cần header `x-reset-token`.
async fn admin_reset(
    State(pool): State<SqlitePool>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let ok = headers
        .get("x-reset-token")
        .and_then(|v| v.to_str().ok())
        .map(|t| t == RESET_TOKEN)
        .unwrap_or(false);
    if !ok {
        return (StatusCode::FORBIDDEN, axum::Json(json!({ "error": "forbidden" }))).into_response();
    }
    db::reset_state(&pool).await;
    axum::Json(json!({ "status": "reset_ok" })).into_response()
}
