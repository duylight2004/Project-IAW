# 📜 Thư Viện Cổ Antiqua

> *"Đằng sau những kệ sách bám bụi của Thư Viện Antiqua là một hệ thống đấu giá thư tịch cổ đã chạy suốt nhiều thập kỷ. Người ta đồn rằng cỗ máy cũ kỹ này giấu ba bí mật — và chỉ những ai đủ kiên nhẫn quan sát mới lấy được chúng."*

CTF giáo dục môn **Hệ quản trị Cơ sở Dữ liệu (DBS)** — một challenge web **3 màn / 3 flag**.

---

## 🎯 Mục tiêu

Thu thập đủ **3 flag** dạng `DBS{...}` ẩn trong hệ thống web của thư viện.

Đây là một challenge web **3 màn nối tiếp**: mỗi màn mở ra một mảnh ghép cần thiết cho màn sau. Đừng vội — thứ bạn thu được ở màn trước chính là chìa khóa cho màn kế tiếp.

---

## 🔗 Truy cập

| | |
|---|---|
| **URL** | `http://localhost:5001` |
| **Tài khoản** | `seeker` / `seeker` |

Đăng nhập để nhận phiên làm việc. Mọi thao tác tra cứu, mua sách và đấu giá đều thực hiện dưới danh nghĩa tài khoản này.

---

## 📖 Luật chơi

- Tìm đủ **3 flag** `DBS{...}`.
- **Ba màn nối tiếp nhau** — thông tin lấy được ở màn trước là điều kiện để vượt qua màn sau. Hãy đọc kỹ *toàn bộ* dữ liệu thu được, không chỉ phần trông giống flag.
- ⚠️ **Có nhiều flag giả** rải rác để gây nhiễu. Tự kiểm chứng đâu là flag thật.
- 🔍 **Quan sát kỹ phản hồi của server** — mã lỗi, nội dung trả về, độ trễ. Manh mối nằm ở *hành vi* của hệ thống, không phải ở gợi ý văn bản trên giao diện.

---

## 🗺️ Khu vực để khám phá

- Trang chủ có ô **tra cứu thư tịch**.
- **Kho sách** (`/shop`) — nơi mua bán, có những cuốn bị khóa.
- **Phòng đấu giá** (`/auction`) — nơi các phiên đấu giá được kết sổ.

---

## ⚖️ Quy tắc

- ✅ Chỉ tấn công vào đúng môi trường challenge được cấp.
- ❌ Không tấn công hạ tầng, không DoS, không phá hoại trạng thái của người chơi khác.
- 🎓 Đây là môi trường giáo dục — các lỗ hổng là **cố ý**. Mục tiêu là học cách nhận diện và khai thác có trách nhiệm.

---

> 💡 **Gợi ý duy nhất:** Một hệ thống cũ thường tin tưởng đầu vào của người dùng nhiều hơn mức an toàn. Hãy thử "nói chuyện" với nó theo cách mà người lập trình không lường trước.

**Chúc bạn săn flag vui vẻ! 🏴**
