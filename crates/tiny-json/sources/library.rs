use std::fmt::Write as _;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
  Null,
  Boolean(bool),
  Number(f64),
  Text(String),
  Array(Vec<Value>),
  Object(Vec<(String, Value)>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
  pub message: String,
  pub position: usize,
}

impl std::fmt::Display for ParseError {
  fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(formatter, "{} (at byte {})", self.message, self.position)
  }
}

impl std::error::Error for ParseError {}

impl Value {
  pub fn get(&self, key: &str) -> Option<&Value> {
    match self {
      Value::Object(entries) => {
        for (entry_key, entry_value) in entries {
          if entry_key == key {
            return Some(entry_value);
          }
        }
        None
      }
      _ => None,
    }
  }

  pub fn as_text(&self) -> Option<&str> {
    match self {
      Value::Text(text) => Some(text.as_str()),
      _ => None,
    }
  }

  pub fn as_number(&self) -> Option<f64> {
    match self {
      Value::Number(number) => Some(*number),
      _ => None,
    }
  }

  pub fn as_integer(&self) -> Option<i64> {
    self.as_number().map(|number| number as i64)
  }

  pub fn as_boolean(&self) -> Option<bool> {
    match self {
      Value::Boolean(boolean) => Some(*boolean),
      _ => None,
    }
  }

  pub fn to_json_string(&self) -> String {
    let mut output = String::new();
    self.write_json(&mut output);
    output
  }

  fn write_json(&self, output: &mut String) {
    match self {
      Value::Null => output.push_str("null"),
      Value::Boolean(boolean) => output.push_str(if *boolean { "true" } else { "false" }),
      Value::Number(number) => write_number(*number, output),
      Value::Text(text) => write_escaped_string(text, output),
      Value::Array(items) => {
        output.push('[');
        for (index, item) in items.iter().enumerate() {
          if index > 0 {
            output.push(',');
          }
          item.write_json(output);
        }
        output.push(']');
      }
      Value::Object(entries) => {
        output.push('{');
        for (index, (key, value)) in entries.iter().enumerate() {
          if index > 0 {
            output.push(',');
          }
          write_escaped_string(key, output);
          output.push(':');
          value.write_json(output);
        }
        output.push('}');
      }
    }
  }
}

pub fn text(value: impl Into<String>) -> Value {
  Value::Text(value.into())
}

pub fn number_from_integer(value: i64) -> Value {
  Value::Number(value as f64)
}

fn write_number(number: f64, output: &mut String) {
  if !number.is_finite() {
    output.push_str("null");
    return;
  }
  if number.fract() == 0.0 && number.abs() < 9.007_199_254_740_992e15 {
    let _ = write!(output, "{}", number as i64);
  } else {
    let _ = write!(output, "{number}");
  }
}

fn write_escaped_string(value: &str, output: &mut String) {
  output.push('"');
  for character in value.chars() {
    match character {
      '"' => output.push_str("\\\""),
      '\\' => output.push_str("\\\\"),
      '\n' => output.push_str("\\n"),
      '\r' => output.push_str("\\r"),
      '\t' => output.push_str("\\t"),
      '\u{08}' => output.push_str("\\b"),
      '\u{0c}' => output.push_str("\\f"),
      control if (control as u32) < 0x20 => {
        let _ = write!(output, "\\u{:04x}", control as u32);
      }
      other => output.push(other),
    }
  }
  output.push('"');
}

pub fn parse(input: &str) -> Result<Value, ParseError> {
  let mut parser = Parser {
    bytes: input.as_bytes(),
    position: 0,
  };
  parser.skip_whitespace();
  let value = parser.parse_value()?;
  parser.skip_whitespace();
  if parser.position != parser.bytes.len() {
    return Err(parser.error("trailing characters after JSON value"));
  }
  Ok(value)
}

struct Parser<'input> {
  bytes: &'input [u8],
  position: usize,
}

impl<'input> Parser<'input> {
  fn error(&self, message: &str) -> ParseError {
    ParseError {
      message: message.to_string(),
      position: self.position,
    }
  }

  fn peek(&self) -> Option<u8> {
    self.bytes.get(self.position).copied()
  }

  fn skip_whitespace(&mut self) {
    while let Some(byte) = self.peek() {
      if byte == b' ' || byte == b'\t' || byte == b'\n' || byte == b'\r' {
        self.position += 1;
      } else {
        break;
      }
    }
  }

  fn parse_value(&mut self) -> Result<Value, ParseError> {
    match self.peek() {
      Some(b'{') => self.parse_object(),
      Some(b'[') => self.parse_array(),
      Some(b'"') => Ok(Value::Text(self.parse_string()?)),
      Some(b't') | Some(b'f') => self.parse_boolean(),
      Some(b'n') => self.parse_null(),
      Some(byte) if byte == b'-' || byte.is_ascii_digit() => self.parse_number(),
      _ => Err(self.error("unexpected character while expecting a value")),
    }
  }

  fn expect_literal(&mut self, literal: &[u8]) -> Result<(), ParseError> {
    if self.position + literal.len() > self.bytes.len() {
      return Err(self.error("unexpected end of input in literal"));
    }
    for (offset, expected) in literal.iter().enumerate() {
      if self.bytes[self.position + offset] != *expected {
        return Err(self.error("invalid literal"));
      }
    }
    self.position += literal.len();
    Ok(())
  }

  fn parse_null(&mut self) -> Result<Value, ParseError> {
    self.expect_literal(b"null")?;
    Ok(Value::Null)
  }

  fn parse_boolean(&mut self) -> Result<Value, ParseError> {
    if self.peek() == Some(b't') {
      self.expect_literal(b"true")?;
      Ok(Value::Boolean(true))
    } else {
      self.expect_literal(b"false")?;
      Ok(Value::Boolean(false))
    }
  }

  fn parse_number(&mut self) -> Result<Value, ParseError> {
    let start = self.position;
    if self.peek() == Some(b'-') {
      self.position += 1;
    }
    while let Some(byte) = self.peek() {
      if byte.is_ascii_digit() {
        self.position += 1;
      } else {
        break;
      }
    }
    if self.peek() == Some(b'.') {
      self.position += 1;
      while let Some(byte) = self.peek() {
        if byte.is_ascii_digit() {
          self.position += 1;
        } else {
          break;
        }
      }
    }
    if let Some(byte) = self.peek()
      && (byte == b'e' || byte == b'E')
    {
      self.position += 1;
      if let Some(sign) = self.peek()
        && (sign == b'+' || sign == b'-')
      {
        self.position += 1;
      }
      while let Some(digit) = self.peek() {
        if digit.is_ascii_digit() {
          self.position += 1;
        } else {
          break;
        }
      }
    }
    let slice = &self.bytes[start..self.position];
    let text = std::str::from_utf8(slice).map_err(|_| self.error("invalid number encoding"))?;
    let parsed = text
      .parse::<f64>()
      .map_err(|_| self.error("malformed number"))?;
    Ok(Value::Number(parsed))
  }

  fn parse_string(&mut self) -> Result<String, ParseError> {
    self.position += 1;
    let mut result = String::new();
    loop {
      let byte = match self.peek() {
        Some(byte) => byte,
        None => return Err(self.error("unterminated string")),
      };
      self.position += 1;
      match byte {
        b'"' => return Ok(result),
        b'\\' => {
          let escape = match self.peek() {
            Some(escape) => escape,
            None => return Err(self.error("unterminated escape sequence")),
          };
          self.position += 1;
          match escape {
            b'"' => result.push('"'),
            b'\\' => result.push('\\'),
            b'/' => result.push('/'),
            b'b' => result.push('\u{08}'),
            b'f' => result.push('\u{0c}'),
            b'n' => result.push('\n'),
            b'r' => result.push('\r'),
            b't' => result.push('\t'),
            b'u' => {
              let code_unit = self.parse_hex_escape()?;
              self.push_unicode(code_unit, &mut result)?;
            }
            _ => return Err(self.error("invalid escape sequence")),
          }
        }
        _ => {
          let character_start = self.position - 1;
          let width = utf8_width(byte);
          let end = character_start + width;
          if end > self.bytes.len() {
            return Err(self.error("invalid UTF-8 in string"));
          }
          let slice = &self.bytes[character_start..end];
          let decoded =
            std::str::from_utf8(slice).map_err(|_| self.error("invalid UTF-8 in string"))?;
          result.push_str(decoded);
          self.position = end;
        }
      }
    }
  }

  fn parse_hex_escape(&mut self) -> Result<u16, ParseError> {
    if self.position + 4 > self.bytes.len() {
      return Err(self.error("incomplete unicode escape"));
    }
    let mut value: u16 = 0;
    for _ in 0..4 {
      let digit = self.bytes[self.position];
      let nibble = match digit {
        b'0'..=b'9' => digit - b'0',
        b'a'..=b'f' => digit - b'a' + 10,
        b'A'..=b'F' => digit - b'A' + 10,
        _ => return Err(self.error("invalid hex digit in unicode escape")),
      };
      value = value * 16 + nibble as u16;
      self.position += 1;
    }
    Ok(value)
  }

  fn push_unicode(&mut self, code_unit: u16, result: &mut String) -> Result<(), ParseError> {
    if (0xD800..=0xDBFF).contains(&code_unit) {
      if self.peek() != Some(b'\\') {
        return Err(self.error("expected low surrogate"));
      }
      self.position += 1;
      if self.peek() != Some(b'u') {
        return Err(self.error("expected low surrogate escape"));
      }
      self.position += 1;
      let low = self.parse_hex_escape()?;
      if !(0xDC00..=0xDFFF).contains(&low) {
        return Err(self.error("invalid low surrogate"));
      }
      let combined = 0x10000 + (((code_unit as u32 - 0xD800) << 10) | (low as u32 - 0xDC00));
      match char::from_u32(combined) {
        Some(character) => result.push(character),
        None => return Err(self.error("invalid surrogate pair")),
      }
    } else {
      match char::from_u32(code_unit as u32) {
        Some(character) => result.push(character),
        None => return Err(self.error("invalid unicode code point")),
      }
    }
    Ok(())
  }

  fn parse_array(&mut self) -> Result<Value, ParseError> {
    self.position += 1;
    let mut items = Vec::new();
    self.skip_whitespace();
    if self.peek() == Some(b']') {
      self.position += 1;
      return Ok(Value::Array(items));
    }
    loop {
      self.skip_whitespace();
      let value = self.parse_value()?;
      items.push(value);
      self.skip_whitespace();
      match self.peek() {
        Some(b',') => {
          self.position += 1;
        }
        Some(b']') => {
          self.position += 1;
          return Ok(Value::Array(items));
        }
        _ => return Err(self.error("expected ',' or ']' in array")),
      }
    }
  }

  fn parse_object(&mut self) -> Result<Value, ParseError> {
    self.position += 1;
    let mut entries: Vec<(String, Value)> = Vec::new();
    self.skip_whitespace();
    if self.peek() == Some(b'}') {
      self.position += 1;
      return Ok(Value::Object(entries));
    }
    loop {
      self.skip_whitespace();
      if self.peek() != Some(b'"') {
        return Err(self.error("expected string key in object"));
      }
      let key = self.parse_string()?;
      self.skip_whitespace();
      if self.peek() != Some(b':') {
        return Err(self.error("expected ':' after object key"));
      }
      self.position += 1;
      self.skip_whitespace();
      let value = self.parse_value()?;
      entries.push((key, value));
      self.skip_whitespace();
      match self.peek() {
        Some(b',') => {
          self.position += 1;
        }
        Some(b'}') => {
          self.position += 1;
          return Ok(Value::Object(entries));
        }
        _ => return Err(self.error("expected ',' or '}' in object")),
      }
    }
  }
}

fn utf8_width(first_byte: u8) -> usize {
  if first_byte < 0x80 {
    1
  } else if first_byte >> 5 == 0b110 {
    2
  } else if first_byte >> 4 == 0b1110 {
    3
  } else if first_byte >> 3 == 0b11110 {
    4
  } else {
    1
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn round_trips_object() -> Result<(), ParseError> {
    let parsed = parse(r#"{"name":"cursor","count":3,"nested":[1,2,3]}"#)?;
    assert_eq!(parsed.get("name").and_then(Value::as_text), Some("cursor"));
    assert_eq!(parsed.get("count").and_then(Value::as_integer), Some(3));
    Ok(())
  }

  #[test]
  fn escapes_output() {
    let value = Value::Text("line\nbreak\"quote".to_string());
    assert_eq!(value.to_json_string(), r#""line\nbreak\"quote""#);
  }

  #[test]
  fn parses_unicode_escape() -> Result<(), ParseError> {
    let parsed = parse(r#""\uD83D\uDE00""#)?;
    assert_eq!(parsed.as_text(), Some("\u{1F600}"));
    Ok(())
  }
}
