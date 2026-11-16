use std::collections::HashMap;

/// Replaces machine-specific values in recorded transcripts while preserving correlations.
pub fn normalize(input: &str, root: &str, pid: u32, version: &str) -> String {
    let mut value = input.replace(root, "<ROOT>");
    value = map_ts_root(&value, "<ROOT>");
    value = replace_release_dir(&value);
    value = replace_pid(&value, pid);
    value = replace_mkdtemp(&value);
    value = replace_uuids(&value);
    value = replace_man_date(&value);
    value = value.replace(version, "<VERSION>");
    value
}

/// Replaces the PID only as a whole number: `1423` and the digits of a UUID stay.
pub fn replace_pid(input: &str, pid: u32) -> String {
    let needle = pid.to_string();
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    while i < input.len() {
        let alnum = |j: usize| bytes.get(j).is_some_and(|b| b.is_ascii_alphanumeric());
        if input[i..].starts_with(&needle) && (i == 0 || !alnum(i - 1)) && !alnum(i + needle.len())
        {
            out.push_str("<PID>");
            i += needle.len();
        } else {
            let ch = input[i..].chars().next().expect("valid UTF-8 boundary");
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

/// An absolute path that ends in the temp root of the TS run (`<tmpdir>/purchasely-cli-XXXXXX`, recorded
/// verbatim in argv and in outputs) becomes `to`: the replay root, or `<ROOT>` in a comparison.
pub fn map_ts_root(value: &str, to: &str) -> String {
    const NAME: &str = "purchasely-cli-";
    let path_char = |c: char| c.is_ascii_alphanumeric() || "/._-+".contains(c);
    let mut out = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(index) = rest.find(NAME) {
        let start = rest[..index]
            .char_indices()
            .rev()
            .take_while(|(_, c)| path_char(*c))
            .last()
            .map_or(index, |(i, _)| i);
        let tail = &rest[index + NAME.len()..];
        let end = tail
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .unwrap_or(tail.len());
        if rest[start..].starts_with('/') && end > 0 {
            out.push_str(&rest[..start]);
            out.push_str(to);
        } else {
            out.push_str(&rest[..index + NAME.len() + end]);
        }
        rest = &tail[end..];
    }
    out.push_str(rest);
    out
}
/// The `mkdtemp` dir of a plugin release (`ai.ts` `fetchPayload`): `cache/ai/release-XXXXXX`.
pub fn replace_release_dir(input: &str) -> String {
    const NAME: &str = "/ai/release-";
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(index) = rest.find(NAME) {
        let tail = &rest[index + NAME.len()..];
        let n = tail.chars().take_while(char::is_ascii_alphanumeric).count();
        out.push_str(&rest[..index + NAME.len()]);
        if n == 6 {
            out.push_str("<MKDTEMP>");
            rest = &tail[n..];
        } else {
            rest = tail;
        }
    }
    out.push_str(rest);
    out
}

pub fn replace_mkdtemp(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(index) = rest.find("purchasely-cli-") {
        out.push_str(&rest[..index]);
        out.push_str("purchasely-cli-<MKDTEMP>");
        rest = &rest[index + "purchasely-cli-".len()..];
        let end = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
            .unwrap_or(rest.len());
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

pub fn replace_uuids(input: &str) -> String {
    let mut ids = HashMap::<String, String>::new();
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if i + 36 <= bytes.len() && is_uuid(&bytes[i..i + 36]) {
            let id = input[i..i + 36].to_ascii_lowercase();
            let next = ids.len() + 1;
            let token = ids.entry(id).or_insert_with(|| format!("<UUID{next}>"));
            out.push_str(token);
            i += 36;
        } else {
            let ch = input[i..].chars().next().expect("valid UTF-8 boundary");
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

fn is_uuid(bytes: &[u8]) -> bool {
    bytes.len() == 36
        && bytes.iter().enumerate().all(|(i, b)| match i {
            8 | 13 | 18 | 23 => *b == b'-',
            _ => b.is_ascii_hexdigit(),
        })
}

pub fn replace_man_date(input: &str) -> String {
    input
        .lines()
        .map(|line| {
            if line.starts_with(".TH ") {
                let mut fields = line.splitn(4, ' ');
                let first = fields.next().unwrap_or_default();
                let second = fields.next().unwrap_or_default();
                let _date = fields.next().unwrap_or_default();
                let tail = fields.next().unwrap_or_default();
                format!("{first} {second} <DATE> {tail}")
                    .trim_end()
                    .to_owned()
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_path_is_replaced() {
        assert_eq!(
            normalize("/tmp/root/a", "/tmp/root", 77, "9.9.9"),
            "<ROOT>/a"
        );
    }
    #[test]
    fn mkdtemp_suffix_is_replaced() {
        assert_eq!(
            replace_mkdtemp("/tmp/purchasely-cli-Ab_123/file"),
            "/tmp/purchasely-cli-<MKDTEMP>/file"
        );
    }
    #[test]
    fn uuid_replacement_preserves_correlation() {
        let id = "123e4567-e89b-42d3-a456-426614174000";
        assert_eq!(replace_uuids(&format!("{id} {id}")), "<UUID1> <UUID1>");
    }
    #[test]
    fn pid_is_replaced() {
        assert_eq!(replace_pid("pid=12345", 12345), "pid=<PID>");
    }
    #[test]
    fn man_date_is_replaced() {
        assert_eq!(
            replace_man_date(".TH FOO 20261116 x\ntext"),
            ".TH FOO <DATE> x\ntext"
        );
    }
    #[test]
    fn version_is_replaced() {
        assert_eq!(
            normalize("version 9.9.9", "/none", 1, "9.9.9"),
            "version <VERSION>"
        );
    }
    #[test]
    fn pid_probe_answers_are_ordered() {
        let probes = [true, false];
        assert_eq!(probes.iter().copied().next(), Some(true));
        assert_eq!(probes.iter().copied().nth(1), Some(false));
    }
}
