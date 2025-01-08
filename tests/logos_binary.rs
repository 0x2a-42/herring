// Copied from https://github.com/maciejhirsz/logos/blob/master/tests/tests/binary.rs
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

#[derive(Herring, Debug, Clone, Copy, PartialEq)]
enum Token {
    #[token("foo")]
    Foo,

    #[regex(b"\x42+")]
    Life,

    #[regex(b"[\xA0-\xAF]+")]
    Aaaaaaa,

    #[token(b"\xCA\xFE\xBE\xEF")]
    CafeBeef,

    #[token(b"\x00")]
    Zero,
}

#[test]
fn handles_non_utf8() {
    assert_lex(
        &[
            0, 0, 0xCA, 0xFE, 0xBE, 0xEF, b'f', b'o', b'o', 0x42, 0x42, 0x42, 0xAA, 0xAA, 0xA2,
            0xAE, 0x10, 0x20, 0,
        ][..],
        &[
            (Ok(Token::Zero), &[0], 0..1),
            (Ok(Token::Zero), &[0], 1..2),
            (Ok(Token::CafeBeef), &[0xCA, 0xFE, 0xBE, 0xEF], 2..6),
            (Ok(Token::Foo), b"foo", 6..9),
            (Ok(Token::Life), &[0x42, 0x42, 0x42], 9..12),
            (Ok(Token::Aaaaaaa), &[0xAA, 0xAA, 0xA2, 0xAE], 12..16),
            (Err(()), &[0x10], 16..17),
            (Err(()), &[0x20], 17..18),
            (Ok(Token::Zero), &[0], 18..19),
        ],
    );
}
