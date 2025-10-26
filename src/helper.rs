/// Truncate `f64` to `usize`
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn f64_to_usize_trunc(x: f64) -> usize {
    debug_assert!(x >= 0.0);
    x.trunc() as usize
}

/// Cast `usize` to `f64`; ignore precision loss
#[allow(clippy::cast_precision_loss)]
pub fn usize_to_f64(x: usize) -> f64 {
    x as f64
}
