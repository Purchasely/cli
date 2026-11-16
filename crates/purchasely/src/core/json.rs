use std::fmt::Write;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(Vec<u16>),
    Array(Vec<Value>),
    Object(Vec<(String, Value)>),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsonError {
    pub offset: usize,
    pub message: &'static str,
}
impl Value {
    pub fn from(s: String) -> Self {
        Self::String(s.encode_utf16().collect())
    }
    pub fn as_str(&self) -> Option<String> {
        if let Self::String(units) = self {
            String::from_utf16(units).ok()
        } else {
            None
        }
    }
}
pub fn parse(input: &str) -> Result<Value, JsonError> {
    let mut p = Parser {
        b: input.as_bytes(),
        i: 0,
    };
    p.ws();
    let v = p.value()?;
    p.ws();
    if p.i != p.b.len() {
        return Err(p.err("trailing characters"));
    }
    Ok(v)
}
pub fn stringify(value: &Value, indent: Option<usize>) -> Result<String, JsonError> {
    let mut out = String::new();
    write_value(value, &mut out, indent, 0)?;
    Ok(out)
}
pub fn stringify_pretty(value: &Value) -> Result<String, JsonError> {
    stringify(value, Some(2))
}
struct Parser<'a> {
    b: &'a [u8],
    i: usize,
}
impl Parser<'_> {
    fn err(&self, m: &'static str) -> JsonError {
        JsonError {
            offset: self.i,
            message: m,
        }
    }
    fn ws(&mut self) {
        while self.i < self.b.len() && matches!(self.b[self.i], b' ' | b'\n' | b'\r' | b'\t') {
            self.i += 1
        }
    }
    fn eat(&mut self, x: u8) -> bool {
        if self.b.get(self.i) == Some(&x) {
            self.i += 1;
            true
        } else {
            false
        }
    }
    fn value(&mut self) -> Result<Value, JsonError> {
        self.ws();
        match self.b.get(self.i).copied() {
            Some(b'n') => {
                self.literal(b"null")?;
                Ok(Value::Null)
            }
            Some(b't') => {
                self.literal(b"true")?;
                Ok(Value::Bool(true))
            }
            Some(b'f') => {
                self.literal(b"false")?;
                Ok(Value::Bool(false))
            }
            Some(b'"') => Ok(Value::String(self.string()?)),
            Some(b'[') => self.array(),
            Some(b'{') => self.object(),
            Some(b'-' | b'0'..=b'9') => self.number(),
            _ => Err(self.err("unexpected token")),
        }
    }
    fn literal(&mut self, s: &[u8]) -> Result<(), JsonError> {
        if self.b.get(self.i..self.i + s.len()) == Some(s) {
            self.i += s.len();
            Ok(())
        } else {
            Err(self.err("invalid literal"))
        }
    }
    fn string(&mut self) -> Result<Vec<u16>, JsonError> {
        self.i += 1;
        let mut out = Vec::new();
        while let Some(&c) = self.b.get(self.i) {
            match c {
                b'"' => {
                    self.i += 1;
                    return Ok(out);
                }
                b'\\' => {
                    self.i += 1;
                    let e = *self
                        .b
                        .get(self.i)
                        .ok_or_else(|| self.err("unterminated escape"))?;
                    self.i += 1;
                    match e {
                        b'"' => out.push(34),
                        b'\\' => out.push(92),
                        b'/' => out.push(47),
                        b'b' => out.push(8),
                        b'f' => out.push(12),
                        b'n' => out.push(10),
                        b'r' => out.push(13),
                        b't' => out.push(9),
                        b'u' => {
                            let hi = self.hex4()?;
                            if (0xd800..=0xdbff).contains(&hi) {
                                if self.b.get(self.i..self.i + 2) == Some(b"\\u") {
                                    self.i += 2;
                                    let lo = self.hex4()?;
                                    if (0xdc00..=0xdfff).contains(&lo) {
                                        out.push(hi);
                                        out.push(lo)
                                    } else {
                                        out.push(hi);
                                        out.push(92);
                                        out.extend("u".encode_utf16());
                                        out.extend(format!("{lo:04x}").encode_utf16())
                                    }
                                } else {
                                    out.push(hi)
                                }
                            } else {
                                out.push(hi)
                            }
                        }
                        _ => return Err(self.err("invalid escape")),
                    }
                }
                0..=31 => return Err(self.err("control character in string")),
                _ => {
                    // Decode one character only: validating the whole tail per character was quadratic.
                    let width = match c {
                        0x00..=0x7f => 1,
                        0xc0..=0xdf => 2,
                        0xe0..=0xef => 3,
                        _ => 4,
                    };
                    let end = (self.i + width).min(self.b.len());
                    let tail = std::str::from_utf8(&self.b[self.i..end])
                        .map_err(|_| self.err("invalid UTF-8"))?;
                    let ch = tail
                        .chars()
                        .next()
                        .ok_or_else(|| self.err("invalid UTF-8"))?;
                    let mut buf = [0; 2];
                    out.extend(ch.encode_utf16(&mut buf).iter().copied());
                    self.i += ch.len_utf8();
                }
            }
        }
        Err(self.err("unterminated string"))
    }
    fn hex4(&mut self) -> Result<u16, JsonError> {
        if self.i + 4 > self.b.len() {
            return Err(self.err("incomplete unicode escape"));
        }
        let mut n = 0u16;
        for &b in &self.b[self.i..self.i + 4] {
            n = n * 16
                + (b as char)
                    .to_digit(16)
                    .ok_or_else(|| self.err("invalid unicode escape"))? as u16
        }
        self.i += 4;
        Ok(n)
    }
    fn array(&mut self) -> Result<Value, JsonError> {
        self.i += 1;
        self.ws();
        let mut a = Vec::new();
        if self.eat(b']') {
            return Ok(Value::Array(a));
        }
        loop {
            a.push(self.value()?);
            self.ws();
            if self.eat(b']') {
                break;
            }
            if !self.eat(b',') {
                return Err(self.err("expected ',' or ']'"));
            }
        }
        Ok(Value::Array(a))
    }
    fn object(&mut self) -> Result<Value, JsonError> {
        self.i += 1;
        self.ws();
        let mut o: Vec<(String, Value)> = Vec::new();
        if self.eat(b'}') {
            return Ok(Value::Object(o));
        }
        loop {
            self.ws();
            if self.b.get(self.i) != Some(&b'"') {
                return Err(self.err("expected property name"));
            }
            let key = String::from_utf16(&self.string()?)
                .map_err(|_| self.err("surrogate in object key"))?;
            self.ws();
            if !self.eat(b':') {
                return Err(self.err("expected ':'"));
            }
            let v = self.value()?;
            if let Some(old) = o.iter_mut().find(|(k, _)| k == &key) {
                old.1 = v
            } else {
                o.push((key, v))
            }
            self.ws();
            if self.eat(b'}') {
                break;
            }
            if !self.eat(b',') {
                return Err(self.err("expected ',' or '}'"));
            }
        }
        Ok(Value::Object(o))
    }
    fn number(&mut self) -> Result<Value, JsonError> {
        let start = self.i;
        self.eat(b'-');
        if !self.eat(b'0') {
            if !matches!(self.b.get(self.i), Some(b'1'..=b'9')) {
                return Err(self.err("invalid number"));
            }
            while matches!(self.b.get(self.i), Some(b'0'..=b'9')) {
                self.i += 1
            }
        }
        if self.eat(b'.') {
            if !matches!(self.b.get(self.i), Some(b'0'..=b'9')) {
                return Err(self.err("invalid number"));
            }
            while matches!(self.b.get(self.i), Some(b'0'..=b'9')) {
                self.i += 1
            }
        }
        if self.eat(b'e') || self.eat(b'E') {
            if !self.eat(b'+') {
                self.eat(b'-');
            }
            if !matches!(self.b.get(self.i), Some(b'0'..=b'9')) {
                return Err(self.err("invalid number"));
            }
            while matches!(self.b.get(self.i), Some(b'0'..=b'9')) {
                self.i += 1
            }
        }
        let s = std::str::from_utf8(&self.b[start..self.i]).unwrap();
        Ok(Value::Number(
            s.parse().map_err(|_| self.err("invalid number"))?,
        ))
    }
}
fn array_index(s: &str) -> Option<u32> {
    if s.is_empty() || s.len() > 10 || (s.len() > 1 && s.starts_with('0')) {
        return None;
    }
    let n = s.parse::<u32>().ok()?;
    if n == u32::MAX || n.to_string() != s {
        None
    } else {
        Some(n)
    }
}
fn quote(s: &[u16], out: &mut String) {
    out.push('"');
    let mut i = 0;
    while i < s.len() {
        let u = s[i];
        match u {
            34 => out.push_str("\\\""),
            92 => out.push_str("\\\\"),
            8 => out.push_str("\\b"),
            12 => out.push_str("\\f"),
            10 => out.push_str("\\n"),
            13 => out.push_str("\\r"),
            9 => out.push_str("\\t"),
            0..=31 => {
                let _ = write!(out, "\\u{u:04x}");
            }
            0xd800..=0xdbff if i + 1 < s.len() && (0xdc00..=0xdfff).contains(&s[i + 1]) => {
                let cp = 0x10000 + (((u as u32 - 0xd800) << 10) | (s[i + 1] as u32 - 0xdc00));
                out.push(char::from_u32(cp).unwrap());
                i += 1
            }
            0xd800..=0xdfff => {
                let _ = write!(out, "\\u{u:04x}");
            }
            _ => out.push(char::from_u32(u as u32).unwrap()),
        }
        i += 1
    }
    out.push('"')
}
fn number(n: f64) -> String {
    if n == 0.0 {
        return "0".into();
    }
    if !n.is_finite() {
        return "null".into();
    }
    let raw = format!("{n:?}");
    let abs = n.abs();
    if (1e-6..1e21).contains(&abs) {
        if raw.contains('e')
            && let Ok(x) = format!("{n:.16}").parse::<f64>()
        {
            let mut s = x.to_string();
            while s.contains('.') && s.ends_with('0') {
                s.pop();
            }
            if s.ends_with('.') {
                s.pop();
            }
            return s;
        }
        return raw.trim_end_matches(".0").into();
    }
    let (mant, exp) = raw.split_once('e').unwrap_or((&raw, "0"));
    let mut m = mant.to_string();
    if m.ends_with(".0") {
        m.truncate(m.len() - 2)
    }
    let e = exp.parse::<i32>().unwrap_or(0);
    format!("{}e{}{e}", m, if e >= 0 { "+" } else { "" })
}
fn write_value(
    v: &Value,
    out: &mut String,
    indent: Option<usize>,
    depth: usize,
) -> Result<(), JsonError> {
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(&number(*n)),
        Value::String(s) => quote(s, out),
        Value::Array(a) => {
            out.push('[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',')
                }
                if let Some(w) = indent {
                    out.push('\n');
                    out.push_str(&" ".repeat(w * (depth + 1)))
                }
                write_value(x, out, indent, depth + 1)?
            }
            if !a.is_empty()
                && let Some(w) = indent
            {
                out.push('\n');
                out.push_str(&" ".repeat(w * depth));
            }
            out.push(']')
        }
        Value::Object(o) => {
            let mut props: Vec<_> = o.iter().collect();
            props.sort_by_key(|(k, _)| array_index(k));
            let mut ints: Vec<_> = props
                .iter()
                .copied()
                .filter(|(k, _)| array_index(k).is_some())
                .collect();
            ints.sort_by_key(|(k, _)| array_index(k));
            let mut rest: Vec<_> = props
                .into_iter()
                .filter(|(k, _)| array_index(k).is_none())
                .collect();
            ints.append(&mut rest);
            out.push('{');
            for (i, (k, x)) in ints.iter().enumerate() {
                if i > 0 {
                    out.push(',')
                }
                if let Some(w) = indent {
                    out.push('\n');
                    out.push_str(&" ".repeat(w * (depth + 1)))
                }
                quote(&k.encode_utf16().collect::<Vec<_>>(), out);
                out.push(':');
                if indent.is_some() {
                    out.push(' ')
                }
                write_value(x, out, indent, depth + 1)?
            }
            if !ints.is_empty()
                && let Some(w) = indent
            {
                out.push('\n');
                out.push_str(&" ".repeat(w * depth));
            }
            out.push('}')
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strings_decode_multibyte_characters() {
        let text = "[\"a\u{e9}\u{20ac}\u{1f600}z\"]";
        assert_eq!(stringify(&parse(text).unwrap(), None).unwrap(), text);
    }
    #[test]
    fn js_number_formatting() {
        assert_eq!(stringify(&parse("1.0").unwrap(), None).unwrap(), "1")
    }
    #[test]
    fn js_exponent_formatting() {
        assert_eq!(stringify(&parse("1e21").unwrap(), None).unwrap(), "1e+21")
    }
    #[test]
    fn preserves_js_object_key_order() {
        let v = parse(r#"{"10":"a","2":"b","x":1}"#).unwrap();
        assert_eq!(stringify(&v, None).unwrap(), r#"{"2":"b","10":"a","x":1}"#)
    }
    #[test]
    fn preserves_lone_surrogate() {
        let v = parse(r#""\ud800""#).unwrap();
        assert_eq!(stringify(&v, None).unwrap(), r#""\ud800""#)
    }
    #[test]
    fn parses_unicode_escape() {
        assert_eq!(
            stringify(&parse(r#""\u0041""#).unwrap(), None).unwrap(),
            r#""A""#
        )
    }
    #[test]
    fn rounds_integer_through_f64() {
        assert_eq!(
            stringify(&parse("9007199254740993").unwrap(), None).unwrap(),
            "9007199254740992"
        )
    }
    #[test]
    fn small_exponent_uses_js_threshold() {
        assert_eq!(stringify(&parse("5e-7").unwrap(), None).unwrap(), "5e-7")
    }
}
