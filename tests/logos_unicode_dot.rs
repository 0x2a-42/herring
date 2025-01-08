// Copied from https://github.com/maciejhirsz/logos/blob/master/tests/tests/unicode_dot.rs
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

use herring::Herring;

#[derive(Herring, Debug, PartialEq)]
enum TestUnicodeDot {
    #[regex(".")]
    Dot,
}

#[test]
fn test_unicode_dot_str_ascii() {
    let mut lexer = TestUnicodeDot::lexer("a");
    assert_eq!(lexer.next(), Some(Ok(TestUnicodeDot::Dot)));
    assert_eq!(lexer.remainder(), "");
    assert_eq!(lexer.next(), None);
}

#[test]
fn test_unicode_dot_str_unicode() {
    let mut lexer = TestUnicodeDot::lexer("");
    assert_eq!(lexer.next(), Some(Ok(TestUnicodeDot::Dot)));
    assert_eq!(lexer.remainder(), "");
    assert_eq!(lexer.next(), None);
}

#[derive(Herring, Debug, PartialEq)]
enum TestUnicodeDotBytes {
    #[regex(".", priority = 100)]
    Dot,
    #[regex(b".", priority = 0)]
    InvalidUtf8,
}

#[test]
fn test_unicode_dot_bytes_ascii() {
    let mut lexer = TestUnicodeDotBytes::lexer(b"a");
    assert_eq!(lexer.next(), Some(Ok(TestUnicodeDotBytes::Dot)));
    assert_eq!(lexer.remainder(), b"");
    assert_eq!(lexer.next(), None);
}

#[test]
fn test_unicode_dot_bytes_unicode() {
    let mut lexer = TestUnicodeDotBytes::lexer("".as_bytes());
    assert_eq!(lexer.next(), Some(Ok(TestUnicodeDotBytes::Dot)));
    assert_eq!(lexer.remainder(), b"");
    assert_eq!(lexer.next(), None);
}

#[test]
fn test_unicode_dot_bytes_invalid_utf8() {
    let mut lexer = TestUnicodeDotBytes::lexer(b"\xff");
    assert_eq!(lexer.next(), Some(Ok(TestUnicodeDotBytes::InvalidUtf8)));
    assert_eq!(lexer.remainder(), b"");
    assert_eq!(lexer.next(), None);
}
