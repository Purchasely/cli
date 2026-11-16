//! Shared 429 backoff policy.
pub const FALLBACK_PAUSE_MS: u64 = 60_000;
pub const MAX_REJECTIONS: usize = 20;
pub fn pause_from_retry_after(value: Option<&str>) -> u64 {
    value
        .and_then(|x| x.parse::<u64>().ok())
        .filter(|x| *x > 0)
        .map(|x| (x * 1000).min(900_000))
        .unwrap_or(FALLBACK_PAUSE_MS)
}
#[cfg(test)]
mod tests {
    #[test]
    fn fallback_pause_is_one_minute() {
        assert_eq!(super::FALLBACK_PAUSE_MS, 60000);
    }
}
