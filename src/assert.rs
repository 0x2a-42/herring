// Copied from https://github.com/maciejhirsz/logos/blob/master/tests/src/lib.rs
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

use super::{Herring, Source, Span};

#[allow(clippy::type_complexity)]
pub fn assert_lex<'a, Token>(
    source: Token::Source,
    tokens: &[(
        Result<Token, Token::Error>,
        <Token::Source as Source<'a>>::Slice,
        Span,
    )],
) where
    Token: Herring<'a> + std::fmt::Debug + PartialEq,
    Token::Extras: Default,
{
    let mut lex = Token::lexer(source);
    for tuple in tokens {
        assert_eq!(
            &(lex.next().expect("Unexpected end"), lex.slice(), lex.span()),
            tuple
        );
    }
    assert_eq!(lex.next(), None);
}
