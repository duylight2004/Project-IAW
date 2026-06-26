use serde::Serialize;
use sqlx::FromRow;

#[derive(Debug, FromRow, Serialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    #[serde(skip_serializing)]
    pub password: String,
    pub coins: i64,
    pub secret_note: Option<String>,
}

#[derive(Debug, FromRow, Serialize)]
pub struct Book {
    pub id: i64,
    pub title: String,
    pub author: String,
    pub price: i64,
    pub restricted: i64,
}

#[derive(Debug, FromRow, Serialize)]
pub struct Bid {
    pub id: i64,
    pub item: String,
    pub status: String,
    pub winner: Option<String>,
}
