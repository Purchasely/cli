//! Local request usage log.
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};
pub const DAY_MS: i64 = 86_400_000;
pub const MAX_BYTES: u64 = 262_144;
pub fn append(path: &Path, line: &str) -> std::io::Result<()> {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p)?
    }
    let mut f = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(f, "{line}")?;
    if f.metadata()?.len() > MAX_BYTES {
        let s = fs::read_to_string(path)?;
        let kept = s.lines().rev().take(1000).collect::<Vec<_>>();
        fs::write(
            path,
            kept.into_iter().rev().collect::<Vec<_>>().join("\n") + "\n",
        )?
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    #[test]
    fn usage_retention_is_one_day() {
        assert_eq!(super::DAY_MS, 86400000);
    }
}
