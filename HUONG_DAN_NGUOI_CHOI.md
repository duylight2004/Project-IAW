# 🧭 Hướng dẫn người chơi — Thư Viện Cổ Antiqua

> Tài liệu này **định hướng cách tiếp cận**, không đưa đáp án trực tiếp. Mỗi màn có các gợi ý xếp theo mức độ — chỉ mở gợi ý sau khi bạn đã thật sự bí ở gợi ý trước.

---

## 🚀 Khởi động

1. Mở app: `http://localhost:5001`
2. Đăng nhập: `seeker` / `seeker`
3. Dạo một vòng các trang: trang chủ (ô tra cứu), `/shop`, `/auction`. Ghi chú lại mọi nút bấm, ô nhập liệu và phản hồi của server.

🛠️ **Bộ công cụ nên có:** trình duyệt + tab Developer Tools (Network), `curl` hoặc Postman để gửi request thủ công, và một chút Python để tính toán.

💡 **Tư duy chung:** Manh mối nằm ở **hành vi** của server (mã lỗi, nội dung trả về, độ trễ), không phải ở chữ gợi ý trên giao diện. Hãy đọc **toàn bộ** dữ liệu thu được — không chỉ phần trông giống `IAW{...}`. Có nhiều **flag giả** để gây nhiễu.

---

## 🟢 Màn 1 — Cánh cổng tra cứu

**Bối cảnh:** Ô tra cứu thư tịch là cửa ngõ đầu tiên. Một hệ thống cũ thường tin tưởng đầu vào người dùng quá mức.

<details><summary>💬 Gợi ý 1 — Bắt đầu từ đâu?</summary>

Thử nhập các **ký tự đặc biệt** vào ô tra cứu thay vì từ khóa bình thường. Quan sát kỹ server phản hồi gì khi gặp ký tự "lạ".
</details>

<details><summary>💬 Gợi ý 2 — Server nói gì?</summary>

Khi gặp một ký tự nhất định, server để lộ **thông báo lỗi nội bộ**. Loại lỗi đó cho bạn biết chính xác dữ liệu của bạn được xử lý như thế nào ở phía sau.
</details>

<details><summary>💬 Gợi ý 3 — Khai thác</summary>

Đây là lỗ hổng kinh điển khi input được ghép thẳng vào câu truy vấn. Bạn cần:
1. Xác định **số cột** mà truy vấn trả về.
2. Ghép thêm dữ liệu từ một bảng khác sao cho **khớp số cột**.
3. Nhắm thẳng vào tài khoản quản trị thay vì lấy bừa — để tránh các flag giả.
</details>

<details><summary>🎁 Bạn nhận được gì?</summary>

**Flag 1** — và quan trọng không kém: một **mã đi kèm** trong cùng dòng dữ liệu. Đừng bỏ qua nó. Mã này là chìa khóa mở Màn 2.

⚠️ **Bẫy:** Đừng phí thời gian tấn công form đăng nhập — nó được bảo vệ đúng cách. Con đường thật nằm ở ô **tra cứu**.
</details>

---

## 🟡 Màn 2 — Cuốn sách bị khóa

**Bối cảnh:** Trong kho có một cuốn sách giá cực đắt mà ví của bạn không đủ mua. Nhưng giá tiền được tính toán... có thể không đáng tin.

<details><summary>💬 Gợi ý 1 — Quan sát</summary>

Thử mua cuốn sách bị khóa với số lượng bình thường. Server trả về cả **tổng giá** và **số dư** của bạn. Giá thay đổi thế nào khi bạn tăng số lượng lên *rất* lớn?
</details>

<details><summary>💬 Gợi ý 2 — Toán học của máy tính</summary>

Số nguyên trong máy tính có **giới hạn**. Khi phép nhân vượt quá giới hạn đó, kết quả sẽ "quay vòng" về số nhỏ. Bạn có thể chọn số lượng sao cho tổng giá quay vòng xuống dưới số dư của mình không?
</details>

<details><summary>💬 Gợi ý 3 — Tính số lượng</summary>

Gọi giá sách là `P`, giới hạn số nguyên là `M` (gợi ý: là một lũy thừa của 2). Bạn cần tìm `quantity` sao cho `(P × quantity) mod M ≤ số dư`. Một chút toán modulo (hoặc duyệt vòng lặp) sẽ ra đáp số.

⚠️ Đừng quên đính kèm **mã bạn lấy được ở Màn 1** vào request — thiếu nó server sẽ từ chối.
</details>

<details><summary>🎁 Bạn nhận được gì?</summary>

**Flag 2** — và một **giấy thông hành** mới trong phản hồi. Lại một lần nữa: nó không phải flag, nhưng là chìa khóa mở Màn 3. Giữ kỹ.

⚠️ **Bẫy tư duy:** Nếu bạn cho rằng "phép nhân tràn số sẽ làm chương trình crash" → hãy thử thật đã. Hệ thống được build ở chế độ tối ưu.
</details>

---

## 🔴 Màn 3 — Phiên đấu giá

**Bối cảnh:** Phòng đấu giá cho phép "kết sổ" một phiên để chốt người thắng. Nhưng việc kết sổ diễn ra qua nhiều bước, và giữa các bước có một khoảng trễ...

<details><summary>💬 Gợi ý 1 — Quan sát độ trễ</summary>

Kết sổ một phiên đấu giá bình thường. Để ý phản hồi của server có nhắc tới một **khoảng thời gian xử lý** tính bằng mili-giây. Điều gì xảy ra *trong* khoảng thời gian đó?
</details>

<details><summary>💬 Gợi ý 2 — Cửa sổ thời gian</summary>

Server **đọc** trạng thái phiên → **chờ** một lúc → mới **ghi** kết quả. Nếu hai yêu cầu cùng lọt vào giữa lúc "đã đọc nhưng chưa ghi", cả hai đều tưởng mình hợp lệ. Đây là một lỗi **race condition (TOCTOU)**.
</details>

<details><summary>💬 Gợi ý 3 — Khai thác</summary>

Bạn cần bắn **2 yêu cầu kết sổ song song** sao cho cả hai cùng lọt vào cửa sổ trễ. Nhưng để ý: mỗi lần kết sổ cần một **vé dùng-một-lần**. Vậy bạn phải xin **2 vé khác nhau** trước, rồi gửi 2 yêu cầu **đồng thời** (không phải tuần tự).

Một chút lập trình bất đồng bộ (async) hoặc đa luồng sẽ giúp bạn gửi song song.

⚠️ Nhớ đính kèm **giấy thông hành từ Màn 2** vào các yêu cầu.
</details>

<details><summary>🎁 Bạn nhận được gì?</summary>

**Flag 3** — server phát hiện một phiên đấu giá có "hai người thắng" và trả về flag cuối cùng.

⚠️ **Bẫy:** Gửi tuần tự → yêu cầu thứ hai thấy phiên đã đóng → thất bại. Dùng lại cùng một vé → bị từ chối. Bắt buộc 2 vé riêng + gửi đồng thời.

🔄 **Làm lại:** Nếu lỡ kết sổ rồi, hãy hỏi người ra đề về cách reset trạng thái để thử lại.
</details>

---

## ✅ Tổng kết

| Màn | Khu vực | Loại lỗ hổng (chỉ mở khi đã hoàn thành) |
|-----|---------|------------------------------------------|
| 1 | Ô tra cứu | <details><summary>?</summary>SQL Injection</details> |
| 2 | Kho sách | <details><summary>?</summary>Integer Overflow</details> |
| 3 | Đấu giá | <details><summary>?</summary>Race Condition (TOCTOU)</details> |

Khi có đủ **3 flag** `IAW{...}` thật (đã loại trừ flag giả), bạn đã hoàn thành challenge. 🏴

> Nhớ: 2 token ẩn (mã ở Màn 1, giấy thông hành ở Màn 2) **không phải flag** — chúng là chìa khóa nối các màn. Đọc kỹ mọi dữ liệu server trả về!
