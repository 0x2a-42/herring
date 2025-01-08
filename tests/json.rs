use herring::{assert_lex, Herring, Lexer};

#[derive(Debug, Clone, PartialEq, Default)]
pub enum LexerError {
    #[default]
    Invalid,
    UnterminatedString,
}
fn parse_string(lexer: &mut Lexer<'_, Token>) -> Result<Token, LexerError> {
    let mut it = lexer.remainder().chars();
    while let Some(c) = it.next() {
        match c {
            '"' => {
                lexer.bump(1);
                return Ok(Token::String);
            }
            '\\' => {
                lexer.bump(1);
                if let Some(c) = it.next() {
                    lexer.bump(c.len_utf8());
                }
            }
            c => lexer.bump(c.len_utf8()),
        }
    }
    Err(LexerError::UnterminatedString)
}
#[derive(Herring, Debug, PartialEq, Copy, Clone)]
#[herring(error = LexerError)]
pub enum Token {
    #[regex("[\u{0020}\u{000A}\u{000D}\u{0009}]+")]
    Whitespace,
    #[token("true")]
    True,
    #[token("false")]
    False,
    #[token("null")]
    Null,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("[")]
    LBrak,
    #[token("]")]
    RBrak,
    #[token(",")]
    Comma,
    #[token(":")]
    Colon,
    #[regex("\"", parse_string)]
    String,
    #[regex(r"-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?")]
    Number,
}

#[test]
fn test_json() {
    assert_lex(
        r#"{"test": [1,2,3]}"#,
        &[
            (Ok(Token::LBrace), "{", 0..1),
            (Ok(Token::String), "\"test\"", 1..7),
            (Ok(Token::Colon), ":", 7..8),
            (Ok(Token::Whitespace), " ", 8..9),
            (Ok(Token::LBrak), "[", 9..10),
            (Ok(Token::Number), "1", 10..11),
            (Ok(Token::Comma), ",", 11..12),
            (Ok(Token::Number), "2", 12..13),
            (Ok(Token::Comma), ",", 13..14),
            (Ok(Token::Number), "3", 14..15),
            (Ok(Token::RBrak), "]", 15..16),
            (Ok(Token::RBrace), "}", 16..17),
        ],
    );
}
