pub fn ntoy(near_amount: u128) -> u128 {
    near_amount * 10u128.pow(24)
}
pub(crate) fn abs_diff_eq(left: u128, right: u128, epsilon: u128) -> bool {
    left <= right + epsilon && right <= left + epsilon
}
