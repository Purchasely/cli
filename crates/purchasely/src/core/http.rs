//! HTTP request policy helpers.
pub const REQUEST_TIMEOUT_MS: u64 = 120_000;
pub fn user_agent(version: &str, reason: &str, command: &str) -> String {
    fn clean(s: &str) -> String {
        s.chars()
            .map(|c| if "\r\n();".contains(c) { ' ' } else { c })
            .collect::<String>()
            .trim()
            .into()
    }
    format!(
        "purchasely-cli/{version} (-; {}; {}; script)",
        clean(reason),
        clean(command)
    )
}
pub fn refused_redirect_exit() -> i32 {
    super::errors::EXIT::REFUSED
}
#[cfg(test)]
mod tests {
    #[test]
    fn user_agent() {
        assert!(super::user_agent("0.7.2", "codex", "list").contains("codex"));
    }
    #[test]
    fn refused_redirect_exit_10() {
        assert_eq!(super::refused_redirect_exit(), 10);
    }
    #[test]
    fn timeout_is_120000() {
        assert_eq!(super::REQUEST_TIMEOUT_MS, 120000);
    }
}
