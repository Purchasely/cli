fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    if m <= 2 {
        y += 1
    }
    (y, m as u32, d as u32)
}
pub fn iso_date(timestamp_ms: i64) -> String {
    let (y, m, d) = civil_from_days(timestamp_ms.div_euclid(86_400_000));
    format!("{y:04}-{m:02}-{d:02}")
}
pub fn iso_datetime(timestamp_ms: i64) -> String {
    let days = timestamp_ms.div_euclid(86_400_000);
    let ms = timestamp_ms.rem_euclid(86_400_000);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.{:03}Z",
        ms / 3_600_000,
        (ms / 60_000) % 60,
        (ms / 1000) % 60,
        ms % 1000
    )
}
#[cfg(test)]
mod tests {
    #[test]
    fn epoch_date_is_iso() {
        assert_eq!(super::iso_date(0), "1970-01-01")
    }
    #[test]
    fn epoch_datetime_is_iso() {
        assert_eq!(super::iso_datetime(0), "1970-01-01T00:00:00.000Z")
    }
    #[test]
    fn negative_time_uses_floor() {
        assert_eq!(super::iso_date(-1), "1969-12-31")
    }
}
