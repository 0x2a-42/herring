// Copied from https://github.com/maciejhirsz/logos/blob/master/tests/tests/custom_error.rs
// and adapted for Herring.
//
// Copyright (c) 2018 Maciej Hirsz <maciej.hirsz@gmail.com>
//
// The MIT License (MIT)
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use herring::{assert_lex, Herring};
use std::num::{IntErrorKind, ParseIntError};

#[derive(Debug, Clone, PartialEq, Default)]
enum LexingError {
    NumberTooLong,
    NumberNotEven(u32),
    #[default]
    Other,
}

impl From<ParseIntError> for LexingError {
    fn from(value: ParseIntError) -> Self {
        match value.kind() {
            IntErrorKind::PosOverflow => LexingError::NumberTooLong,
            _ => LexingError::Other,
        }
    }
}

fn parse_number(input: &str) -> Result<Token, LexingError> {
    let num = input.parse::<u32>()?;
    if num % 2 == 0 {
        Ok(Token::Number)
    } else {
        Err(LexingError::NumberNotEven(num))
    }
}

#[derive(Herring, Debug, Clone, Copy, PartialEq)]
#[herring(error = LexingError)]
enum Token {
    #[regex(r"[0-9]+", |lex| parse_number(lex.slice()))]
    Number,
    #[regex(r"[a-zA-Z_]+")]
    Identifier,
}

#[test]
fn test() {
    assert_lex(
        "123abc1234xyz1111111111111111111111111111111111111111111111111111111,",
        &[
            (Err(LexingError::NumberNotEven(123)), "123", 0..3),
            (Ok(Token::Identifier), "abc", 3..6),
            (Ok(Token::Number), "1234", 6..10),
            (Ok(Token::Identifier), "xyz", 10..13),
            (
                Err(LexingError::NumberTooLong),
                "1111111111111111111111111111111111111111111111111111111",
                13..68,
            ),
            (Err(LexingError::Other), ",", 68..69),
        ],
    );
}
