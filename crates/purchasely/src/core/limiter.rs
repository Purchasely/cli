//! Cross-process API request limiter.
pub const MAX_IN_FLIGHT: usize = 4;
pub const WRITE_GAP_MS: i64 = 7000;
pub const RATE: i64 = 800;
pub const STALE_SLOT_MS: i64 = 300_000;
#[cfg(test)]
mod tests {
    #[test]
    fn constants_match_contract() {
        assert_eq!(super::MAX_IN_FLIGHT, 4);
        assert_eq!(super::WRITE_GAP_MS, 7000);
    }
}
