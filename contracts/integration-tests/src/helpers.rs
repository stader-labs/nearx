const CLOSE_DELTA: u128 = 5_000_000_000_000_000_000_000_0; // Delta for now

pub fn ntoy(near_amount: u128) -> u128 {
    near_amount * 10u128.pow(24)
}

pub fn is_close(a: u128, b: u128) -> bool {
    let delta = a - b;

    delta.le(&CLOSE_DELTA)
}
