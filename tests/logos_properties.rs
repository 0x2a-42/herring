// Copied from https://github.com/maciejhirsz/logos/blob/master/tests/tests/properties.rs
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
#[herring(skip r"[ \t\n\f]+")]
enum Token {
    #[regex(r"[a-zA-Z]+")]
    Ascii,

    #[regex(r"\p{Greek}+")]
    Greek,

    #[regex(r"\p{Cyrillic}+")]
    Cyrillic,
}

#[test]
fn greek() {
    assert_lex(
        "λόγος can do unicode",
        &[
            (Ok(Token::Greek), "λόγος", 0..10),
            (Ok(Token::Ascii), "can", 11..14),
            (Ok(Token::Ascii), "do", 15..17),
            (Ok(Token::Ascii), "unicode", 18..25),
        ],
    )
}

#[test]
fn cyrillic() {
    assert_lex(
        "До свидания",
        &[
            (Ok(Token::Cyrillic), "До", 0..4),
            (Ok(Token::Cyrillic), "свидания", 5..21),
        ],
    )
}
