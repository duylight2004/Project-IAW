use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
use argon2::Argon2;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::str::FromStr;

// Flag 1 — nằm trong DB (users.secret_note của admin), lấy qua SQLi UNION ở /search.
const FLAG1: &str = "FLAG{l3gacy_s34rch_un10n_1nj3ct10n}";

/// argon2id hash của một mật khẩu (salt ngẫu nhiên mỗi lần seed).
fn hash_password(plain: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(plain.as_bytes(), &salt)
        .expect("argon2 hash")
        .to_string()
}

/// Mở (tạo nếu chưa có) pool SQLite tới file `url`.
pub async fn init_pool(url: &str) -> SqlitePool {
    let opts = SqliteConnectOptions::from_str(url)
        .expect("sqlite url")
        .create_if_missing(true);
    SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(opts)
        .await
        .expect("connect sqlite")
}

/// Tạo schema (idempotent) rồi seed nếu bảng users còn rỗng.
pub async fn init_db(pool: &SqlitePool) {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id          INTEGER PRIMARY KEY,
            username    TEXT NOT NULL UNIQUE,
            password    TEXT NOT NULL,
            coins       INTEGER NOT NULL DEFAULT 1000,
            secret_note TEXT
        );
        CREATE TABLE IF NOT EXISTS books (
            id         INTEGER PRIMARY KEY,
            title      TEXT NOT NULL,
            author     TEXT NOT NULL,
            price      INTEGER NOT NULL,
            restricted INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS purchases (
            id      INTEGER PRIMARY KEY,
            user_id INTEGER NOT NULL,
            book_id INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS bids (
            id     INTEGER PRIMARY KEY,
            item   TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'open',
            winner TEXT
        );
        CREATE TABLE IF NOT EXISTS settlements (
            id     INTEGER PRIMARY KEY,
            bid_id INTEGER NOT NULL,
            who    TEXT NOT NULL
        );
        "#,
    )
    .execute(pool)
    .await
    .expect("create schema");

    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await
        .unwrap_or(0);
    if n == 0 {
        seed(pool).await;
    }
}

/// Reset trạng thái Act 2/3 (purchases, bids, settlements) về ban đầu.
/// Dùng cho /admin/reset để test lặp; KHÔNG đụng tới users/flag.
pub async fn reset_state(pool: &SqlitePool) {
    let _ = sqlx::query("DELETE FROM purchases").execute(pool).await;
    let _ = sqlx::query("DELETE FROM settlements").execute(pool).await;
    let _ = sqlx::query("UPDATE users SET coins = 1000 WHERE username = 'seeker'")
        .execute(pool)
        .await;
    let _ = sqlx::query("UPDATE bids SET status = 'open', winner = NULL")
        .execute(pool)
        .await;
}

async fn seed(pool: &SqlitePool) {
    // admin: secret_note giữ FLAG 1
    sqlx::query("INSERT INTO users (username, password, coins, secret_note) VALUES (?, ?, ?, ?)")
        .bind("admin")
        .bind(hash_password("S0_l0ng_4nd_th4nks_f0r_4ll"))
        .bind(99999_i64)
        .bind(FLAG1)
        .execute(pool)
        .await
        .expect("seed admin");

    // người chơi mặc định
    sqlx::query("INSERT INTO users (username, password, coins, secret_note) VALUES (?, ?, ?, NULL)")
        .bind("seeker")
        .bind(hash_password("seeker"))
        .bind(1000_i64)
        .execute(pool)
        .await
        .expect("seed seeker");

    // sách thường (lấp danh mục + cho /search trả kết quả lành tính)
    let normal = [
        ("Bản Đồ Sao Cổ", "V. Andronikos", 40),
        ("Thảo Mộc Học Phương Đông", "L. Trần", 25),
        ("Hồi Ký Người Đóng Sách", "M. Đặng", 30),
        ("Niên Giám Hải Hành 1742", "P. Nguyễn", 55),
    ];
    for (t, a, p) in normal {
        sqlx::query("INSERT INTO books (title, author, price, restricted) VALUES (?, ?, ?, 0)")
            .bind(t)
            .bind(a)
            .bind(p as i64)
            .execute(pool)
            .await
            .expect("seed book");
    }

    // sách restricted — Act 2 (price = 9999 lẻ -> mọi remainder reachable)
    sqlx::query("INSERT INTO books (title, author, price, restricted) VALUES (?, ?, ?, 1)")
        .bind("Mật Lục Antiqua [locked]")
        .bind("Khuyết Danh")
        .bind(9999_i64)
        .execute(pool)
        .await
        .expect("seed restricted book");

    // bid mở sẵn — Act 3
    sqlx::query("INSERT INTO bids (item, status, winner) VALUES (?, 'open', NULL)")
        .bind("Bản thảo gốc Antiqua")
        .execute(pool)
        .await
        .expect("seed bid");
}
