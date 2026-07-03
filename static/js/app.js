// Tra cứu thư tịch — trang chủ CHỈ hiển thị một dòng trạng thái chung, không
// liệt kê bản ghi và không mở modal. Muốn đọc dữ liệu rò rỉ (Act 1 SQLi),
// người chơi phải tự soi response gốc qua DevTools -> Network hoặc curl/Postman.
// Backend vẫn giữ nguyên lỗ hổng: cả kết quả lẫn thông báo lỗi SQL vẫn nằm đầy
// đủ trong HTTP response, chỉ là giao diện không phơi ra.
async function doSearch() {
  const query = document.getElementById('q').value;
  const out = document.getElementById('out');
  const r = await fetch(`/search?q=${encodeURIComponent(query)}`);
  const data = await r.json();

  out.textContent = buildMessage(data);
}

// Trả về một dòng trạng thái đơn giản — không bao giờ lộ nội dung bản ghi
// hay chi tiết lỗi SQL.
function buildMessage(data) {
  if (data && data.error) {
    // Chi tiết lỗi SQL vẫn nằm trong response (tab Network) để enumerate cột.
    return '⚠ Kho lưu trữ gặp sự cố khi xử lý truy vấn tìm kiếm.';
  }

  const results = (data && data.results) || [];
  if (results.length === 0) {
    return 'Không tìm thấy thư tịch nào khớp với từ khoá.';
  }
  return `Tìm thấy ${results.length} thư tịch trong kho lưu trữ.`;
}
