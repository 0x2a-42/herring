use herring_automata::Nfa;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use std::collections::{HashMap, HashSet};
use syn::parse::{Parse, ParseStream};
use syn::{
    parenthesized, parse2, Error, Expr, ExprClosure, ExprPath, Fields, Ident, ItemEnum, Lit,
    LitInt, LitStr, Token, Type, Variant,
};

use crate::generate::SKIP_NAME;

fn consume_comma(input: ParseStream) -> bool {
    input
        .parse::<Option<Token![,]>>()
        .is_ok_and(|comma| comma.is_some())
}
fn peek_ident(input: ParseStream, name: &str) -> bool {
    input
        .fork()
        .parse::<Ident>()
        .is_ok_and(|ident| ident == name)
}

struct SubpatternParse {
    name: String,
    pattern: String,
}
impl Parse for SubpatternParse {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        if ident != "subpattern" {
            return Err(Error::new(ident.span(), "unexpected property"));
        }
        let name = input.parse::<Ident>()?.to_string();
        let _assign: Token![=] = input.parse()?;
        let pattern = input.parse::<LitStr>()?.value();
        Ok(Self { name, pattern })
    }
}

struct FuncRefParse(Expr);
impl Parse for FuncRefParse {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if let Ok(callback) = input.parse::<ExprPath>() {
            Ok(Self(Expr::Path(callback)))
        } else if let Ok(callback) = input.parse::<ExprClosure>() {
            Ok(Self(Expr::Closure(callback)))
        } else {
            Err(Error::new(
                input.span(),
                "expected path to function or closure expression",
            ))
        }
    }
}

struct PriorityParse {
    value: usize,
}
impl Parse for PriorityParse {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.fork().parse()?;
        if ident != "priority" {
            return Err(input.error("expected `priority`"));
        } else {
            let _ = input.parse::<Ident>();
        }
        let _assign: Token![=] = input.parse()?;
        let value: LitInt = input.parse()?;
        Ok(Self {
            value: value.to_string().parse().unwrap(),
        })
    }
}

struct IgnoreParse {
    case: bool,
}
impl Parse for IgnoreParse {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.fork().parse()?;
        if ident != "ignore" {
            return Err(input.error("expected `ignore`"));
        } else {
            let _ = input.parse::<Ident>();
        }
        let content;
        parenthesized!(content in input);
        let flag: Ident = content.parse()?;
        if flag == "case" {
            Ok(Self {
                case: flag == "case",
            })
        } else {
            Err(Error::new(flag.span(), "unsupported ignore flag"))
        }
    }
}

fn bytes_to_regex(bytes: &[u8]) -> String {
    let mut res = String::new();
    for b in bytes {
        if *b <= 0x7f {
            res.push(*b as char);
        } else {
            res.push_str(&format!("\\x{b:x?}"));
        }
    }
    res
}

struct RegexParse {
    regex: String,
    bytes: Vec<u8>,
    span: Span,
    callback: Option<Expr>,
    priority: Option<usize>,
    ignore_case: bool,
    binary: bool,
}
impl Parse for RegexParse {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let (regex, bytes, span, binary) = match input.parse::<Lit>()? {
            Lit::Str(lit_str) => (lit_str.value(), Vec::new(), lit_str.span(), false),
            Lit::ByteStr(lit_byte_str) => (
                bytes_to_regex(&lit_byte_str.value()),
                lit_byte_str.value(),
                lit_byte_str.span(),
                true,
            ),
            lit => return Err(Error::new(lit.span(), "expected string or byte string")),
        };
        let mut callback = None;
        let mut priority = None;
        let mut ignore_case = false;
        if consume_comma(input) {
            if peek_ident(input, "priority") {
                priority = Some(input.parse::<PriorityParse>()?.value);
                if input.parse::<Option<Token![,]>>()?.is_some() {
                    ignore_case = input.parse::<IgnoreParse>()?.case;
                }
            } else if peek_ident(input, "ignore") {
                ignore_case = input.parse::<IgnoreParse>()?.case;
            } else {
                let funcref = input.parse::<FuncRefParse>()?;
                callback = Some(funcref.0);
                if consume_comma(input) {
                    if peek_ident(input, "priority") {
                        priority = Some(input.parse::<PriorityParse>()?.value);
                        if consume_comma(input) {
                            ignore_case = input.parse::<IgnoreParse>()?.case;
                        }
                    } else {
                        ignore_case = input.parse::<IgnoreParse>()?.case;
                    }
                }
            }
        }
        Ok(Self {
            regex,
            bytes,
            span,
            callback,
            priority,
            ignore_case,
            binary,
        })
    }
}

struct SkipParse(RegexParse);

impl Parse for SkipParse {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        if ident != "skip" {
            return Err(Error::new(ident.span(), "unexpected property"));
        }
        Ok(SkipParse(input.parse::<RegexParse>()?))
    }
}

pub(crate) struct EnumAttrs {
    pub(crate) extras_ty: Type,
    pub(crate) error_ty: Type,
    pub(crate) source_ty: TokenStream,
    pub(crate) ignore_cb: Option<Expr>,
    pub(crate) initial_cb: Option<Expr>,
    pub(crate) subpatterns: HashMap<String, String>,
}
fn parse_enum_attrs(
    item: &ItemEnum,
    tokens: &mut Vec<herring_automata::Token>,
    regex_set: &mut HashSet<(String, bool, bool)>,
    callbacks: &mut HashMap<(String, usize), Expr>,
    binary: &mut bool,
) -> syn::Result<EnumAttrs> {
    let mut extras_ty = Type::Verbatim(quote! {()});
    let mut error_ty = Type::Verbatim(quote! {()});
    let mut source_ty = quote! {};
    let mut ignore_cb: Option<Expr> = None;
    let mut initial_cb: Option<Expr> = None;
    let mut subpatterns = HashMap::new();
    let mut used_attrs = HashSet::new();
    let mut number = 0;
    for attr in item.attrs.iter() {
        if attr.path().is_ident("herring") {
            let property_result = attr.parse_nested_meta(|meta| {
                if let Some(ident) = meta.path.get_ident() {
                    let name = ident.to_string();
                    if name != "subpattern" && used_attrs.contains(&name) {
                        return Err(Error::new(
                            ident.span(),
                            format!("{name} was already specified"),
                        ));
                    }
                    match name.as_str() {
                        "extras" => extras_ty = meta.value()?.parse()?,
                        "error" => error_ty = meta.value()?.parse()?,
                        "ignore" => ignore_cb = Some(meta.value()?.parse::<FuncRefParse>()?.0),
                        "initial" => initial_cb = Some(meta.value()?.parse::<FuncRefParse>()?.0),
                        "source" => {
                            let ty = meta.value()?.parse::<Type>()?;
                            source_ty = quote! {&'source #ty };
                        }
                        _ => return Err(Error::new(ident.span(), "unexpected property")),
                    }
                    used_attrs.insert(ident.to_string());
                }
                Ok(())
            });
            if let Err(err) = property_result {
                if let Ok(subpattern) = attr.parse_args::<SubpatternParse>() {
                    subpatterns.insert(subpattern.name, subpattern.pattern);
                } else if let Ok(skip) = attr.parse_args::<SkipParse>() {
                    *binary |= skip.0.binary;
                    let number = if let Some(callback) = skip.0.callback {
                        number += 1;
                        callbacks.insert((SKIP_NAME.to_string(), number), callback);
                        number
                    } else {
                        0
                    };
                    check_duplicate(
                        "regex",
                        regex_set,
                        &skip.0.regex,
                        skip.0.ignore_case,
                        skip.0.binary,
                        skip.0.span,
                    )?;
                    let (nfa, prio) = match Nfa::from_regex_with_subpatterns(
                        &skip.0.regex,
                        &subpatterns,
                        skip.0.ignore_case,
                        skip.0.binary,
                    ) {
                        Ok((nfa, prio)) => (nfa, prio),
                        Err(err) => {
                            return Err(Error::new(skip.0.span, err.message));
                        }
                    };
                    let prio = skip.0.priority.unwrap_or(prio);
                    if nfa.accepts_empty() {
                        return Err(Error::new(skip.0.span, "skip regex matches empty word"));
                    }
                    tokens.push(herring_automata::Token::new(
                        nfa,
                        prio,
                        (SKIP_NAME.to_string(), number),
                    ));
                } else {
                    return Err(err);
                }
            }
        }
    }
    Ok(EnumAttrs {
        extras_ty,
        error_ty,
        source_ty,
        ignore_cb,
        initial_cb,
        subpatterns,
    })
}

fn check_duplicate(
    kind: &str,
    set: &mut HashSet<(String, bool, bool)>,
    value: &str,
    ignore_case: bool,
    binary: bool,
    span: Span,
) -> syn::Result<()> {
    if set.contains(&(value.to_string(), ignore_case, binary)) {
        return Err(Error::new(
            span,
            format!(
                "identical {} \"{}\" was already used",
                kind,
                value.escape_debug()
            ),
        ));
    } else {
        set.insert((value.to_string(), ignore_case, binary));
    }
    Ok(())
}

fn parse_variant_attrs(
    variant: &Variant,
    tokens: &mut Vec<herring_automata::Token>,
    token_set: &mut HashSet<(String, bool, bool)>,
    regex_set: &mut HashSet<(String, bool, bool)>,
    callbacks: &mut HashMap<(String, usize), Expr>,
    subpatterns: &HashMap<String, String>,
    binary: &mut bool,
) -> syn::Result<()> {
    let mut number = 0;
    for attr in variant.attrs.iter() {
        if let Some(ident) = attr.path().get_ident() {
            let name = ident.to_string();
            let tok = variant.ident.to_string();
            let parse = attr.parse_args::<RegexParse>()?;
            *binary |= parse.binary;
            let number = if let Some(callback) = parse.callback {
                if variant.attrs.len() > 1 {
                    number += 1;
                }
                callbacks.insert((tok.clone(), number), callback);
                number
            } else {
                0
            };
            let (nfa, prio) = match name.as_str() {
                "token" => {
                    check_duplicate(
                        &name,
                        token_set,
                        &parse.regex,
                        parse.ignore_case,
                        parse.binary,
                        parse.span,
                    )?;
                    if parse.binary {
                        (
                            Nfa::from_bytes(&parse.bytes, parse.ignore_case),
                            parse.bytes.len() * 2,
                        )
                    } else {
                        Nfa::from_token(&parse.regex, parse.ignore_case)
                    }
                }
                "regex" => {
                    check_duplicate(
                        &name,
                        regex_set,
                        &parse.regex,
                        parse.ignore_case,
                        parse.binary,
                        parse.span,
                    )?;
                    match Nfa::from_regex_with_subpatterns(
                        &parse.regex,
                        subpatterns,
                        parse.ignore_case,
                        parse.binary,
                    ) {
                        Ok((nfa, prio)) => (nfa, prio),
                        Err(err) => {
                            return Err(Error::new(parse.span, err.message));
                        }
                    }
                }
                _ => return Err(Error::new(ident.span(), "expected `token` or `regex`")),
            };
            let prio = parse.priority.unwrap_or(prio);
            if nfa.accepts_empty() {
                return Err(Error::new(parse.span, "token regex matches empty word"));
            }
            tokens.push(herring_automata::Token::new(nfa, prio, (tok, number)));
        }
    }
    Ok(())
}

pub(crate) struct EnumVariants {
    pub(crate) tokens: Vec<herring_automata::Token>,
    pub(crate) callbacks: HashMap<(String, usize), Expr>,
}
fn parse_enum_variants(
    item: &ItemEnum,
    subpatterns: &HashMap<String, String>,
    mut tokens: Vec<herring_automata::Token>,
    regex_set: &mut HashSet<(String, bool, bool)>,
    mut callbacks: HashMap<(String, usize), Expr>,
    binary: &mut bool,
) -> syn::Result<EnumVariants> {
    let mut token_set = HashSet::new();
    for variant in item.variants.iter() {
        let span = variant.ident.span();
        match variant.fields {
            Fields::Named(_) | Fields::Unnamed(_) => {
                return Err(Error::new(span, "Herring only supports unit variants"));
            }
            Fields::Unit => {}
        }
        parse_variant_attrs(
            variant,
            &mut tokens,
            &mut token_set,
            regex_set,
            &mut callbacks,
            subpatterns,
            binary,
        )?;
    }
    Ok(EnumVariants { tokens, callbacks })
}

pub(crate) struct Enum {
    pub(crate) name: Ident,
    pub(crate) attrs: EnumAttrs,
    pub(crate) variants: EnumVariants,
    pub(crate) binary: bool,
}
pub(crate) fn parse_enum(tokens: TokenStream) -> syn::Result<Enum> {
    let item = match parse2::<ItemEnum>(tokens) {
        Ok(item) => item,
        Err(err) => {
            return Err(Error::new(
                err.span(),
                "Herring trait can only be derived on enums",
            ));
        }
    };
    let name = item.ident.clone();
    let mut tokens = vec![];
    let mut regex_set = HashSet::new();
    let mut callbacks = HashMap::new();
    let mut binary = false;
    let attrs = parse_enum_attrs(
        &item,
        &mut tokens,
        &mut regex_set,
        &mut callbacks,
        &mut binary,
    )?;
    let variants = parse_enum_variants(
        &item,
        &attrs.subpatterns,
        tokens,
        &mut regex_set,
        callbacks,
        &mut binary,
    )?;
    Ok(Enum {
        name,
        attrs,
        variants,
        binary,
    })
}
