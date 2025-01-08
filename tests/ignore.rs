use herring::{assert_lex, Herring, Lexer};

fn ignore_escaped_newline(lexer: &mut Lexer<'_, Token>) {
    if lexer.remainder().starts_with("\\\n") {
        lexer.bump(2);
    }
}

#[derive(Herring, Debug, PartialEq, Copy, Clone)]
#[herring(skip "[ \n]+")]
#[herring(ignore = ignore_escaped_newline)]
pub enum Token {
    #[regex("[a-zA-Z][a-zA-Z_0-9]*")]
    Identifier,
}

#[test]
fn test_escaped_newline() {
    assert_lex(
        "foo\n b\\\nar",
        &[
            (Ok(Token::Identifier), "foo", 0..3),
            (Ok(Token::Identifier), "b\\\nar", 5..10),
        ],
    );
}
