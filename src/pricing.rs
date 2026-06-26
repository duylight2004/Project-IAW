// src/pricing.rs — LỖ HỔNG CỐ Ý (Act 2)
//
// fast arithmetic, no checks needed in prod
//
// KHÔNG checked_mul, KHÔNG wrapping_mul — chỉ phép `*` thường.
//   - release build (overflow-checks = false): wraps quanh 2^32  -> khai thác được
//   - debug build  (overflow-checks = true):  panic 'attempt to multiply with overflow'
//
// quantity là input runtime nên KHÔNG bị const-fold; trình biên dịch không thể
// đánh giá trước -> không có compile error, phép nhân chạy đúng theo profile.
pub fn total_price(base_price: u32, quantity: u32) -> u32 {
    base_price * quantity
}
