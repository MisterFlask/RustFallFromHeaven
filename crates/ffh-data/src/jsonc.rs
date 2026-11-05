//! JSON with comments: the format of the game data (`data/json/*.jsonc`).
//!
//! JSON (RFC 8259) plus `//` line comments, `/* */` block comments and a trailing comma after the
//! last member of an object or array. Objects keep their members in file order (a `Vec`), and a
//! key may appear only once in an object. Integers (no fraction, no exponent) are [`Value::Int`],
//! other numbers [`Value::Float`]. std only (crates.io is unreachable).

use std::fmt;

/// A parsed value.
#[derive(Clone, Debug, PartialEq, Default)]
pub enum Value {
    #[default]
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Arr(Vec<Value>),
    Obj(Vec<(String, Value)>),
}

impl Value {
    /// The member `key` of an object (`None` for a missing key or a value that is no object).
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Obj(m) => m.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_obj(&self) -> Option<&[(String, Value)]> {
        match self {
            Value::Obj(m) => Some(m),
            _ => None,
        }
    }

    pub fn as_arr(&self) -> Option<&[Value]> {
        match self {
            Value::Arr(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// A number as a float (an integer converts).
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Int(i) => Some(*i as f64),
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// The kind's name, for messages.
    pub fn kind(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "a boolean",
            Value::Int(_) => "an integer",
            Value::Float(_) => "a number",
            Value::Str(_) => "a string",
            Value::Arr(_) => "an array",
            Value::Obj(_) => "an object",
        }
    }
}

/// Where and why parsing failed (line and column from 1).
#[derive(Clone, Debug, PartialEq)]
pub struct ParseError {
    pub line: usize,
    pub col: usize,
    pub msg: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}, column {}: {}", self.line, self.col, self.msg)
    }
}

impl std::error::Error for ParseError {}

/// Parses a whole document (one value, then only whitespace and comments).
pub fn parse(src: &str) -> Result<Value, ParseError> {
    let src = src.trim_start_matches('\u{feff}');
    let mut p = Parser { s: src.as_bytes(), src, pos: 0 };
    p.skip()?;
    let v = p.value(0)?;
    p.skip()?;
    if p.pos < p.s.len() {
        return p.err("unexpected content after the value");
    }
    Ok(v)
}

const MAX_DEPTH: usize = 256;

struct Parser<'s> {
    s: &'s [u8],
    src: &'s str,
    pos: usize,
}

impl Parser<'_> {
    fn err<T>(&self, msg: impl Into<String>) -> Result<T, ParseError> {
        let before = &self.src[..self.pos.min(self.src.len())];
        let line = before.matches('\n').count() + 1;
        let col = before.rsplit('\n').next().map_or(0, |l| l.chars().count()) + 1;
        Err(ParseError { line, col, msg: msg.into() })
    }

    fn peek(&self) -> Option<u8> {
        self.s.get(self.pos).copied()
    }

    /// Skips whitespace and comments.
    fn skip(&mut self) -> Result<(), ParseError> {
        loop {
            match self.peek() {
                Some(b' ' | b'\t' | b'\n' | b'\r') => self.pos += 1,
                Some(b'/') if self.s.get(self.pos + 1) == Some(&b'/') => {
                    while self.peek().is_some_and(|c| c != b'\n') {
                        self.pos += 1;
                    }
                }
                Some(b'/') if self.s.get(self.pos + 1) == Some(&b'*') => match self.src[self.pos + 2..].find("*/") {
                    Some(i) => self.pos += 2 + i + 2,
                    None => return self.err("unterminated /* comment"),
                },
                _ => return Ok(()),
            }
        }
    }

    fn value(&mut self, depth: usize) -> Result<Value, ParseError> {
        if depth > MAX_DEPTH {
            return self.err("nested too deeply");
        }
        match self.peek() {
            Some(b'{') => self.object(depth),
            Some(b'[') => self.array(depth),
            Some(b'"') => Ok(Value::Str(self.string()?)),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(_) => {
                for (word, v) in [("true", Value::Bool(true)), ("false", Value::Bool(false)), ("null", Value::Null)] {
                    if self.s[self.pos..].starts_with(word.as_bytes()) {
                        self.pos += word.len();
                        return Ok(v);
                    }
                }
                self.err("expected a value")
            }
            None => self.err("unexpected end, expected a value"),
        }
    }

    fn object(&mut self, depth: usize) -> Result<Value, ParseError> {
        self.pos += 1;
        let mut members: Vec<(String, Value)> = Vec::new();
        loop {
            self.skip()?;
            match self.peek() {
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(Value::Obj(members));
                }
                Some(b'"') => {}
                _ => return self.err("expected a key or '}'"),
            }
            let key_pos = self.pos;
            let key = self.string()?;
            if members.iter().any(|(k, _)| *k == key) {
                self.pos = key_pos;
                return self.err(format!("duplicate key {key:?}"));
            }
            self.skip()?;
            if self.peek() != Some(b':') {
                return self.err("expected ':'");
            }
            self.pos += 1;
            self.skip()?;
            let v = self.value(depth + 1)?;
            members.push((key, v));
            self.skip()?;
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b'}') => {}
                _ => return self.err("expected ',' or '}'"),
            }
        }
    }

    fn array(&mut self, depth: usize) -> Result<Value, ParseError> {
        self.pos += 1;
        let mut items = Vec::new();
        loop {
            self.skip()?;
            if self.peek() == Some(b']') {
                self.pos += 1;
                return Ok(Value::Arr(items));
            }
            items.push(self.value(depth + 1)?);
            self.skip()?;
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b']') => {}
                _ => return self.err("expected ',' or ']'"),
            }
        }
    }

    fn string(&mut self) -> Result<String, ParseError> {
        self.pos += 1;
        let mut out = String::new();
        loop {
            let start = self.pos;
            while self.peek().is_some_and(|c| c != b'"' && c != b'\\' && c >= 0x20) {
                self.pos += 1;
            }
            out.push_str(&self.src[start..self.pos]);
            match self.peek() {
                Some(b'"') => {
                    self.pos += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    self.pos += 1;
                    let c = match self.peek() {
                        Some(b'"') => '"',
                        Some(b'\\') => '\\',
                        Some(b'/') => '/',
                        Some(b'b') => '\u{8}',
                        Some(b'f') => '\u{c}',
                        Some(b'n') => '\n',
                        Some(b'r') => '\r',
                        Some(b't') => '\t',
                        Some(b'u') => {
                            self.pos += 1;
                            let hi = self.hex4()?;
                            let code = if (0xD800..0xDC00).contains(&hi) && self.s[self.pos..].starts_with(b"\\u") {
                                self.pos += 2;
                                let lo = self.hex4()?;
                                0x10000 + ((hi - 0xD800) << 10) + (lo.wrapping_sub(0xDC00) & 0x3FF)
                            } else {
                                hi
                            };
                            out.push(char::from_u32(code).unwrap_or('\u{fffd}'));
                            continue;
                        }
                        _ => return self.err("bad escape in a string"),
                    };
                    self.pos += 1;
                    out.push(c);
                }
                Some(_) => return self.err("control character in a string"),
                None => return self.err("unterminated string"),
            }
        }
    }

    fn hex4(&mut self) -> Result<u32, ParseError> {
        let h = self.src.get(self.pos..self.pos + 4).and_then(|h| u32::from_str_radix(h, 16).ok());
        match h {
            Some(v) => {
                self.pos += 4;
                Ok(v)
            }
            None => self.err("bad \\u escape"),
        }
    }

    fn number(&mut self) -> Result<Value, ParseError> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        let mut float = false;
        while let Some(c) = self.peek() {
            match c {
                b'0'..=b'9' => {}
                b'.' | b'e' | b'E' | b'+' | b'-' => float = true,
                _ => break,
            }
            self.pos += 1;
        }
        let text = &self.src[start..self.pos];
        if !float {
            if let Ok(i) = text.parse::<i64>() {
                return Ok(Value::Int(i));
            }
        }
        match text.parse::<f64>() {
            Ok(f) if f.is_finite() => Ok(Value::Float(f)),
            _ => {
                self.pos = start;
                self.err(format!("bad number {text:?}"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_comments_trailing_commas_and_order() {
        let v = parse("// head\n{ \"b\": 1, /* x */ \"a\": [1.5, \"s\\n\", true, null,], }\n// tail").unwrap();
        let m = v.as_obj().unwrap();
        assert_eq!(m[0], ("b".to_string(), Value::Int(1)));
        assert_eq!(m[1].1, Value::Arr(vec![Value::Float(1.5), Value::Str("s\n".into()), Value::Bool(true), Value::Null]));
    }

    #[test]
    fn rejects_duplicate_keys_and_garbage() {
        let e = parse("{\n  \"a\": 1,\n  \"a\": 2\n}").unwrap_err();
        assert_eq!((e.line, e.col), (3, 3));
        assert!(parse("[1 2]").is_err());
        assert!(parse("{\"a\": 1} x").is_err());
        assert!(parse("\"\\ud83d\\ude00\"").unwrap() == Value::Str("\u{1F600}".into()));
    }
}
