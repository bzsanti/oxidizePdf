//! Static CIDSystemInfo declarations only; this is not a PostScript interpreter.
use super::encoding_cmap::EncodingCMap;

#[derive(PartialEq)]
enum Token<'a> {
    Name(&'a [u8]),
    Word(&'a [u8]),
    String(Vec<u8>),
    Open(u8),
    Close(u8),
}

struct Scanner<'a> {
    bytes: &'a [u8],
    pos: usize,
}

fn delimiter(b: u8) -> bool {
    whitespace(b) || b"()<>[]{}/%".contains(&b)
}

fn whitespace(b: u8) -> bool {
    b == 0 || b.is_ascii_whitespace()
}

impl<'a> Scanner<'a> {
    fn next(&mut self) -> Result<Option<Token<'a>>, ()> {
        loop {
            while self.bytes.get(self.pos).is_some_and(|b| whitespace(*b)) {
                self.pos += 1;
            }
            if self.bytes.get(self.pos) != Some(&b'%') {
                break;
            }
            while self
                .bytes
                .get(self.pos)
                .is_some_and(|b| !matches!(b, b'\r' | b'\n'))
            {
                self.pos += 1;
            }
        }
        let Some(&b) = self.bytes.get(self.pos) else {
            return Ok(None);
        };
        self.pos += 1;
        let token = match b {
            b'(' => {
                let mut depth = 1usize;
                let mut value = Vec::new();
                while depth > 0 {
                    let b = *self.bytes.get(self.pos).ok_or(())?;
                    self.pos += 1;
                    match b {
                        b'(' => {
                            depth += 1;
                            value.push(b);
                        }
                        b')' => {
                            depth -= 1;
                            if depth > 0 {
                                value.push(b);
                            }
                        }
                        b'\\' => {
                            let escaped = *self.bytes.get(self.pos).ok_or(())?;
                            self.pos += 1;
                            match escaped {
                                b'\n' => {}
                                b'\r' => {
                                    if self.bytes.get(self.pos) == Some(&b'\n') {
                                        self.pos += 1;
                                    }
                                }
                                b'0'..=b'7' => {
                                    let mut octal = u16::from(escaped - b'0');
                                    for _ in 0..2 {
                                        let Some(&digit @ b'0'..=b'7') = self.bytes.get(self.pos)
                                        else {
                                            break;
                                        };
                                        octal = octal * 8 + u16::from(digit - b'0');
                                        self.pos += 1;
                                    }
                                    value.push(octal as u8);
                                }
                                _ => value.push(match escaped {
                                    b'n' => b'\n',
                                    b'r' => b'\r',
                                    b't' => b'\t',
                                    b'b' => 8,
                                    b'f' => 12,
                                    _ => escaped,
                                }),
                            }
                        }
                        _ => value.push(b),
                    }
                }
                Token::String(value)
            }
            b'<' if self.bytes.get(self.pos) == Some(&b'<') => {
                self.pos += 1;
                Token::Open(b'<')
            }
            b'>' if self.bytes.get(self.pos) == Some(&b'>') => {
                self.pos += 1;
                Token::Close(b'<')
            }
            b'<' => {
                let mut value = Vec::new();
                let mut high = None;
                loop {
                    let b = *self.bytes.get(self.pos).ok_or(())?;
                    self.pos += 1;
                    if b == b'>' {
                        break;
                    }
                    if whitespace(b) {
                        continue;
                    }
                    let digit = (b as char).to_digit(16).ok_or(())? as u8;
                    if let Some(h) = high.take() {
                        value.push(h * 16 + digit);
                    } else {
                        high = Some(digit);
                    }
                }
                if let Some(h) = high {
                    value.push(h * 16);
                }
                Token::String(value)
            }
            b'{' | b'[' => Token::Open(b),
            b'}' => Token::Close(b'{'),
            b']' => Token::Close(b'['),
            b')' | b'>' => return Err(()),
            _ => {
                let start = if b == b'/' { self.pos } else { self.pos - 1 };
                while self.bytes.get(self.pos).is_some_and(|b| !delimiter(*b)) {
                    self.pos += 1;
                }
                let value = &self.bytes[start..self.pos];
                if b == b'/' {
                    Token::Name(value)
                } else {
                    Token::Word(value)
                }
            }
        };
        Ok(Some(token))
    }

    fn word(&mut self, word: &[u8]) -> Result<(), ()> {
        if self.next()? == Some(Token::Word(word)) {
            Ok(())
        } else {
            Err(())
        }
    }
}

fn collection(scanner: &mut Scanner<'_>) -> Result<(String, String), ()> {
    let conventional = match scanner.next()? {
        Some(Token::Open(b'<')) => false,
        Some(Token::Word(size))
            if std::str::from_utf8(size)
                .ok()
                .and_then(|s| s.parse::<u32>().ok())
                .is_some() =>
        {
            scanner.word(b"dict")?;
            scanner.word(b"dup")?;
            scanner.word(b"begin")?;
            true
        }
        _ => return Err(()),
    };
    let (mut registry, mut ordering, mut supplement) = (None, None, None);
    loop {
        let key = match scanner.next()? {
            Some(Token::Close(b'<')) if !conventional => break,
            Some(Token::Word(b"end")) if conventional => break,
            Some(Token::Name(key)) => key,
            _ => return Err(()),
        };
        match (key, scanner.next()?) {
            (b"Registry", Some(Token::String(value))) => {
                registry = Some(String::from_utf8(value).map_err(|_| ())?)
            }
            (b"Ordering", Some(Token::String(value))) => {
                ordering = Some(String::from_utf8(value).map_err(|_| ())?)
            }
            (b"Supplement", Some(Token::Word(value))) => {
                supplement = std::str::from_utf8(value)
                    .ok()
                    .and_then(|s| s.parse::<i64>().ok())
                    .filter(|n| *n >= 0);
                if supplement.is_none() {
                    return Err(());
                }
            }
            _ => return Err(()),
        }
        if conventional {
            scanner.word(b"def")?;
        }
    }
    let mut end = scanner.next()?;
    if end == Some(Token::Word(b"readonly")) {
        end = scanner.next()?;
    }
    if end != Some(Token::Word(b"def")) || supplement.is_none() {
        return Err(());
    }
    Ok((registry.ok_or(())?, ordering.ok_or(())?))
}

/// Only declarations at the active CMap dictionary level carry authority.
pub(super) fn apply(data: &[u8], map: &mut EncodingCMap) {
    let mut scanner = Scanner {
        bytes: data,
        pos: 0,
    };
    let mut containers = Vec::new();
    let mut begins = 0usize;
    let mut active = false;
    loop {
        let token = match scanner.next() {
            Ok(Some(token)) => token,
            Ok(None) => break,
            Err(()) => {
                map.invalid_collection = true;
                break;
            }
        };
        match token {
            Token::Open(kind) => containers.push(kind),
            Token::Close(kind) => {
                if containers.pop() != Some(kind) {
                    map.invalid_collection = true;
                    break;
                }
            }
            _ if !containers.is_empty() => {}
            Token::Word(b"begincmap") => {
                active = true;
                begins = 0;
            }
            Token::Word(b"endcmap") => break,
            Token::Word(b"begin") if active => begins += 1,
            Token::Word(b"end") if active => begins = begins.saturating_sub(1),
            Token::Name(b"CIDSystemInfo") if active && begins == 0 => {
                match collection(&mut scanner) {
                    Ok(value) => map.declare_collection(Some(value)),
                    Err(()) => {
                        map.invalid_collection = true;
                        break;
                    }
                }
            }
            _ => {}
        }
    }
}
