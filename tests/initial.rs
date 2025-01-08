use herring::{assert_lex, Herring, Lexer};

fn check_indent(lexer: &mut Lexer<'_, Token>) {
    let indent = (lexer.slice().len() - 1 - lexer.slice().rfind('\n').unwrap()) / 4;
    if indent > lexer.extras.indent {
        lexer.extras.pending_indent = indent - lexer.extras.indent;
    } else if indent < lexer.extras.indent {
        lexer.extras.pending_dedent = lexer.extras.indent - indent;
    }
    lexer.extras.indent = indent;
}
fn emit_indent_dedent(lexer: &mut Lexer<'_, Token>) -> Option<Result<Token, ()>> {
    if lexer.extras.pending_indent > 0 {
        lexer.extras.pending_indent -= 1;
        return Some(Ok(Token::Indent));
    }
    if lexer.extras.pending_dedent > 0 {
        lexer.extras.pending_dedent -= 1;
        return Some(Ok(Token::Dedent));
    }
    None
}

#[derive(Default)]
pub struct Context {
    indent: usize,
    pending_indent: usize,
    pending_dedent: usize,
}

#[derive(Herring, Debug, PartialEq, Copy, Clone)]
#[herring(initial = emit_indent_dedent)]
#[herring(extras = Context)]
#[herring(skip "(\n *)+", check_indent)]
#[herring(skip " +")]
pub enum Token {
    #[regex("[a-zA-Z_][a-zA-Z_0-9]*")]
    Identifier,
    #[token("def")]
    Def,
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("True")]
    True,
    #[token("pass")]
    Pass,
    #[token(":")]
    Colon,
    #[token("(")]
    LPar,
    #[token(")")]
    RPar,
    Indent,
    Dedent,
}

#[test]
fn test_indent_dedent() {
    assert_lex(
        "def foo():\
       \n    if True:\
       \n        bar()\
       \n\
       \n        pass\
       \n    else:\
       \n        pass\n",
        &[
            (Ok(Token::Def), "def", 0..3),
            (Ok(Token::Identifier), "foo", 4..7),
            (Ok(Token::LPar), "(", 7..8),
            (Ok(Token::RPar), ")", 8..9),
            (Ok(Token::Colon), ":", 9..10),
            (Ok(Token::Indent), "", 15..15),
            (Ok(Token::If), "if", 15..17),
            (Ok(Token::True), "True", 18..22),
            (Ok(Token::Colon), ":", 22..23),
            (Ok(Token::Indent), "", 32..32),
            (Ok(Token::Identifier), "bar", 32..35),
            (Ok(Token::LPar), "(", 35..36),
            (Ok(Token::RPar), ")", 36..37),
            (Ok(Token::Pass), "pass", 47..51),
            (Ok(Token::Dedent), "", 56..56),
            (Ok(Token::Else), "else", 56..60),
            (Ok(Token::Colon), ":", 60..61),
            (Ok(Token::Indent), "", 70..70),
            (Ok(Token::Pass), "pass", 70..74),
            (Ok(Token::Dedent), "", 75..75),
            (Ok(Token::Dedent), "", 75..75),
        ],
    );
}
