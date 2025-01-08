#![forbid(unsafe_code)]

mod assert;

pub use assert::assert_lex;
pub use herring_derive::Herring;

pub type Span = core::ops::Range<usize>;

pub trait Source<'source> {
    type Slice: PartialEq + Eq + core::fmt::Debug;

    fn get_byte(&mut self, offset: usize) -> Option<u8>;
    fn remainder(&self, offset: usize) -> Self::Slice;
    fn slice(&self, start: usize, end: usize) -> Self::Slice;
    fn is_boundary(&self, offset: usize) -> bool;
}

impl<'source> Source<'source> for &'source [u8] {
    type Slice = Self;

    #[inline(always)]
    fn get_byte(&mut self, offset: usize) -> Option<u8> {
        self.get(offset).copied()
    }
    #[inline(always)]
    fn remainder(&self, offset: usize) -> Self::Slice {
        &self[offset..]
    }
    #[inline(always)]
    fn slice(&self, start: usize, end: usize) -> Self::Slice {
        &self[start..end]
    }
    #[inline(always)]
    fn is_boundary(&self, _offset: usize) -> bool {
        true
    }
}
impl<'source> Source<'source> for &'source str {
    type Slice = Self;

    #[inline(always)]
    fn get_byte(&mut self, offset: usize) -> Option<u8> {
        self.as_bytes().get(offset).copied()
    }
    #[inline(always)]
    fn remainder(&self, offset: usize) -> Self::Slice {
        &self[offset..]
    }
    #[inline(always)]
    fn slice(&self, start: usize, end: usize) -> Self::Slice {
        &self[start..end]
    }
    #[inline(always)]
    fn is_boundary(&self, offset: usize) -> bool {
        self.is_char_boundary(offset)
    }
}

pub trait Herring<'source>: Sized {
    type Error: Default + Clone + PartialEq + core::fmt::Debug;
    type Extras;
    type Source: Source<'source>;

    fn ignore(lexer: &mut Lexer<'source, Self>);
    fn lex(lexer: &mut Lexer<'source, Self>) -> Option<Result<Self, Self::Error>>;
    fn lexer(source: Self::Source) -> Lexer<'source, Self>
    where
        Self::Extras: Default,
    {
        Lexer::new(source)
    }
}

pub struct Lexer<'source, Token: Herring<'source>> {
    pub start: usize,
    pub offset: usize,
    pub source: Token::Source,
    pub extras: Token::Extras,
}

impl<'source, Token: Herring<'source>> Lexer<'source, Token> {
    pub fn new(source: Token::Source) -> Self
    where
        Token::Extras: Default,
    {
        Self {
            start: 0,
            offset: 0,
            source,
            extras: Default::default(),
        }
    }
    pub fn with_extras(source: Token::Source, extras: Token::Extras) -> Self {
        Self {
            start: 0,
            offset: 0,
            source,
            extras,
        }
    }
    #[inline(always)]
    pub fn next_byte(&mut self) -> Option<u8> {
        Token::ignore(self);
        let offset = self.offset;
        self.offset += 1;
        self.source.get_byte(offset)
    }
    #[inline(always)]
    pub fn bump(&mut self, n: usize) {
        self.offset += n;
    }
    #[inline(always)]
    pub fn remainder(&self) -> <Token::Source as Source<'source>>::Slice {
        self.source.remainder(self.offset)
    }
    #[inline(always)]
    pub fn slice(&self) -> <Token::Source as Source<'source>>::Slice {
        self.source.slice(self.start, self.offset)
    }
    #[inline(always)]
    pub fn span(&self) -> Span {
        self.start..self.offset
    }
    #[inline(always)]
    pub fn spanned(self) -> SpannedIter<'source, Token> {
        SpannedIter { lexer: self }
    }
}

impl<'source, Token: Herring<'source>> Iterator for Lexer<'source, Token> {
    type Item = Result<Token, Token::Error>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        Token::lex(self)
    }
}

pub struct SpannedIter<'source, Token: Herring<'source>> {
    lexer: Lexer<'source, Token>,
}

impl<'source, Token: Herring<'source>> Iterator for SpannedIter<'source, Token> {
    type Item = (Result<Token, Token::Error>, Span);

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        Token::lex(&mut self.lexer).map(|tok| (tok, self.lexer.span()))
    }
}
