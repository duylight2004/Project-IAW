# Thư Viện Cổ Antiqua — CTF giáo dục (IAW)

Một challenge web **3 Act / 3 Flag** viết bằng **Rust** (`axum` + `sqlx` + SQLite), dùng cho môn *An toàn & Bảo mật Thông tin*. Mỗi Act minh hoạ một lớp lỗ hổng kinh điển trong tình huống "thư viện đấu giá thư tịch cổ".

> ⚠️ **Chỉ dùng cho mục đích giáo dục** trong môi trường được phép. Các lỗ hổng ở đây là **cố ý**. Không triển khai ra Internet công khai.

---

## Lỗ hổng theo từng Act

| Act | Chủ đề | Lớp lỗ hổng | Endpoint | Flag nằm ở |
|-----|--------|-------------|----------|------------|
| **1** | "Cánh cổng giả" | **SQL Injection** (UNION, do nối chuỗi `format!` không bind) | `GET /search?q=` | `users.secret_note` của **admin** trong DB |
| **2** | "Giá sách ma thuật" | **Integer overflow** `u32` (release build, không `checked_mul`) | `POST /buy` | Render trong handler khi mua thành công sách restricted |
| **3** | "Đồng hồ đếm ngược" | **Race condition / TOCTOU** (read → window → write, không atomic) | `POST /bids/settle` | Server trả khi phát hiện double-settle |

**Rabbit hole cố ý:** `POST /login` dùng prepared statement chuẩn → SQLi tại login luôn thất bại. Con đường thật của Act 1 là `/search`.

**Mồi nhử (traps):** trong DB rải nhiều chuỗi `IAW{f4k3_fl4g...}` (note của `seeker`/`librarian`, một bid đã đóng). Chỉ `secret_note` của `admin` là flag thật.

---

## Yêu cầu

- [Docker](https://docs.docker.com/) + Docker Compose, **hoặc**
- Rust toolchain (chỉ khi build thủ công). ⚠️ **Bắt buộc build `--release`** — Act 2 dựa vào `overflow-checks = false` của profile release; build debug sẽ *panic* khi nhân tràn số.

---

## Chạy nhanh

### Docker Compose (khuyên dùng)

```bash
docker compose up --build
# App lắng nghe tại http://localhost:5001  (8080 bên trong container)
```

Mỗi container = 1 người chơi/team. DB nằm trên `tmpfs` (`/tmp/db`) và được tạo mới mỗi lần khởi động, rootfs read-only, chạy UID 1000.

### Kubernetes

```bash
kubectl apply -f challenge.yaml   # Deployment + Service (ClusterIP :5001 -> 8080)
```

### Tài khoản mặc định

| User | Mật khẩu | Coins | Ghi chú |
|------|----------|-------|---------|
| `seeker` | `seeker` | 1000 | tài khoản người chơi công khai |

> Đăng nhập tạo cookie `antiqua_session`. Các thao tác mua/đấu giá đều thực hiện dưới danh nghĩa `seeker`.

---

## Bản đồ endpoint

| Method | Path | Vai trò |
|--------|------|---------|
| GET | `/` | Trang chủ + ô tra cứu |
| GET·POST | `/login` | Form đăng nhập (rabbit hole, prepared stmt) |
| GET | `/shop` | Trang Act 2 — danh mục + mua sách (cần đăng nhập) |
| GET | `/auction` | Trang Act 3 — settle phiên đấu giá (cần đăng nhập) |
| GET | `/search?q=` | **Act 1** — tra cứu (SQLi) |
| GET | `/catalog` | Danh mục sách (JSON) |
| POST | `/buy` | **Act 2** — mua sách (integer overflow) |
| POST | `/bids/settle` | **Act 3** — kết thúc đấu giá (TOCTOU) |
| POST | `/admin/reset` | Hạ tầng — reset state Act 2/3 (header `x-reset-token`) |

---

## Lời giải mẫu

Script khai thác cả 3 Act nằm trong [`exploits/`](exploits/) (Python 3, stdlib thuần, không cần `pip install`):

```bash
cd exploits
python solve_all.py                      # chạy cả 3 Act, in 3 flag
python solve_act1.py http://localhost:5001
ANTIQUA_URL=http://10.0.0.5:5001 python solve_act2.py
```

Chi tiết xem [`exploits/README.md`](exploits/README.md).

### Reset khi test lặp (chủ yếu cho Act 3)

```bash
curl -X POST http://localhost:5001/admin/reset -H 'x-reset-token: antiqua-reset-7f3a'
```

---

## Cấu trúc thư mục

```
antiqua-library/
├── Dockerfile            # multi-stage: rust:slim (build --release --locked) -> debian:slim
├── docker-compose.yml    # map 5001:8080, read-only rootfs + tmpfs /tmp/db
├── challenge.yaml        # K8s Deployment + Service (UID 1000, readOnlyRootFS)
├── entrypoint.sh         # tạo /tmp/db, reset DB, exec binary
├── Cargo.toml            # axum, sqlx(sqlite), tokio, serde, argon2 | [profile.release] overflow-checks=false
├── src/
│   ├── main.rs           # router + state(pool), bind 0.0.0.0:8080
│   ├── db.rs             # pool, init_db(), seed(), reset_state()
│   ├── models.rs         # User / Book / Bid (FromRow)
│   ├── pricing.rs        # total_price() — overflow cố ý (Act 2)
│   └── routes/
│       ├── auth.rs       # /login prepared stmt (rabbit hole)
│       ├── books.rs      # /search SQLi (Act 1) + /buy overflow (Act 2)
│       └── bids.rs       # /bids/settle TOCTOU (Act 3)
├── templates/            # index / login / shop / auction (HTML, có hint)
├── static/               # css/style.css, js/app.js, js/modal.js
└── exploits/             # solve_act{1,2,3}.py, solve_all.py, common.py
```

---

## Ghi chú vận hành

- **Act 2 cần `--release`.** Dockerfile đã ép `cargo build --release --locked`. Nếu build tay, đừng dùng debug.
- **Cách ly flag:** dump DB qua Act 1 **không** lộ flag của Act 2/3 (hai flag đó là hằng trong code, render khi đạt điều kiện, không nằm trong DB).
- **Tài liệu thiết kế nội bộ** (walkthrough đầy đủ, dành cho người ra đề — **không phát cho người chơi**): [`SPEC.md`](SPEC.md).
