use crate::parse::*;
use herring_automata::{Dfa, Nfa, Output, Pattern, State, StateRef, Transition};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use std::collections::{BTreeMap, HashMap};
use syn::{Error, Expr, Ident};

pub(crate) const SKIP_NAME: &str = "skipped regex";

fn generate_dfa(tokens: Vec<herring_automata::Token>) -> syn::Result<Dfa> {
    let nfa = Nfa::new_tokenizer(tokens);
    crate::debug::graph(&nfa, "nfa")?;

    let subset_dfa = match nfa.into_dfa() {
        Ok(dfa) => dfa,
        Err(err) => return Err(Error::new(Span::call_site(), err.message)),
    };
    crate::debug::graph(&subset_dfa, "dfa")?;

    let minimal_dfa = subset_dfa.into_minimized();
    crate::debug::graph(&minimal_dfa, "min")?;

    Ok(minimal_dfa)
}

macro_rules! ident {
    ($fmt:literal) => {
        Ident::new(&format!($fmt), Span::call_site())
    };
    ($fmt:literal, $($args:expr),*) => {
        Ident::new(&format!($fmt, $($args),*), Span::call_site())
    };
    ($s:expr) => {
        Ident::new(&$s, Span::call_site())
    };
}

fn generate_stacked_lut_pattern<'a>(
    pattern: &'a Pattern,
    luts: &mut BTreeMap<&'a Pattern, usize>,
) -> TokenStream {
    let index = if let Some(index) = luts.get(pattern) {
        *index
    } else {
        let index = luts.len();
        luts.insert(pattern, index);
        index
    };
    let lut_ident = ident!("LUT{}", index / 8);
    let mask = 1u8 << (index % 8);

    quote! { Some(b) if #lut_ident[b as usize] & #mask > 0 }
}

fn generate_byte_pattern(pattern: &Pattern) -> TokenStream {
    let mut ranges = vec![];
    for r in pattern.ranges().iter() {
        let start = r.start();
        let end = r.end();
        if start == end {
            ranges.push(quote! { #start });
        } else {
            ranges.push(quote! { #start ..= #end });
        }
    }
    quote! { Some(#(#ranges)|*) }
}

fn generate_pattern<'a>(
    transition: &'a Transition,
    luts: &mut BTreeMap<&'a Pattern, usize>,
) -> TokenStream {
    let range_count = transition
        .when()
        .ranges()
        .iter()
        .filter(|r| r.start() != r.end())
        .count();
    let single_count = transition
        .when()
        .ranges()
        .iter()
        .filter(|r| r.start() == r.end())
        .count();
    if range_count > 1 || (range_count == 1 && single_count > 0) {
        generate_stacked_lut_pattern(transition.when(), luts)
    } else {
        generate_byte_pattern(transition.when())
    }
}

fn generate_eof_jump(dfa: &Dfa, state_ref: StateRef) -> TokenStream {
    if dfa.start() == state_ref {
        quote! {
           lexer.offset -= 1;
           return None;
        }
    } else {
        quote! {
           lexer.offset -= 1;
           break 'fsm;
        }
    }
}

fn generate_pattern_transitions<'a>(
    dfa: &'a Dfa,
    state_ref: StateRef,
    state: &'a State,
    luts: &mut BTreeMap<&'a Pattern, usize>,
) -> TokenStream {
    let mut transitions = vec![];
    for transition in state.transitions().iter() {
        let next_state = ident!("S{}", transition.to().value());
        let condition = generate_pattern(transition, luts);
        let jump = if transition.to() == state_ref {
            quote! { continue; }
        } else {
            quote! {
                state = State::#next_state;
                continue 'fsm;
            }
        };
        transitions.push(quote! {
           #condition => {
               #jump
           }
        });
    }
    let eof_jump = generate_eof_jump(dfa, state_ref);
    transitions.push(quote! { None => { #eof_jump } });
    quote! {
        match lexer.next_byte() {
            #(#transitions)*
            _ => break 'fsm,
        }
    }
}

fn generate_lut_transitions(dfa: &Dfa, state_ref: StateRef, state: &State) -> TokenStream {
    let mut entries = vec![];
    'outer: for b in u8::MIN..=u8::MAX {
        for t in state.transitions().iter() {
            if t.when().contains(b) {
                let ident = ident!("J{}", t.to().value());
                entries.push(quote! { #ident });
                continue 'outer;
            }
        }
        entries.push(quote! { __ });
    }
    let eof_jump = generate_eof_jump(dfa, state_ref);
    let mut targets = vec![];
    let mut jumps = vec![];
    for t in state.transitions().iter() {
        targets.push(ident!("J{}", t.to().value()));
        jumps.push(if t.to() == state_ref {
            quote! { continue; }
        } else {
            let state_ident = ident!("S{}", t.to().value());
            quote! {
                state = State::#state_ident;
                continue 'fsm;
            }
        });
    }
    quote! {
        #[derive(Clone, Copy)]
        enum Jumps {
            #(#targets,)*
            __,
        }
        const LUT: [Jumps; 256] = {
            use Jumps::*;
            [
                #(#entries),*
            ]
        };
        if let Some(byte) = lexer.next_byte() {
            match LUT[byte as usize] {
                #(Jumps::#targets => {
                    #jumps
                })*
                Jumps::__ => break 'fsm,
            }
        }
        #eof_jump
    }
}

fn generate_transitions<'a>(
    dfa: &'a Dfa,
    state_ref: StateRef,
    state: &'a State,
    luts: &mut BTreeMap<&'a Pattern, usize>,
) -> TokenStream {
    if state.transitions().len() >= 3
        && state
            .transitions()
            .iter()
            .any(|t| t.when().ranges().iter().any(|p| p.start() != p.end()))
    {
        generate_lut_transitions(dfa, state_ref, state)
    } else {
        generate_pattern_transitions(dfa, state_ref, state, luts)
    }
}

fn generate_callback_def(
    enum_name: &Ident,
    output: &Option<Output>,
    callbacks: &HashMap<(String, usize), Expr>,
    is_skip: bool,
) -> TokenStream {
    if let Some(output) = output {
        if let Some(callback) = callbacks.get(output.value()) {
            return if is_skip {
                quote! {
                    let callback: fn(&mut herring::Lexer<'source, #enum_name>) = #callback;
                }
            } else {
                quote! {
                    let callback: fn(
                        &mut herring::Lexer<'source, #enum_name>
                    ) -> Result<#enum_name, <Self as Herring<'source>>::Error> = #callback;
                }
            };
        }
    }
    quote! {}
}

fn generate_last_accept(
    callback_def: &TokenStream,
    output: &Option<Output>,
    enum_name: &Ident,
    is_skip: bool,
) -> TokenStream {
    if let Some(output) = output {
        if callback_def.is_empty() {
            if is_skip {
                quote! { last_accept = LastAccept::Skip(lexer.offset); }
            } else {
                let enumerator = ident!(output.value().0);
                quote! { last_accept = LastAccept::Token(#enum_name::#enumerator, lexer.offset); }
            }
        } else if is_skip {
            quote! { last_accept = LastAccept::SkipCallback(callback, lexer.offset); }
        } else {
            quote! { last_accept = LastAccept::TokenCallback(callback, lexer.offset); }
        }
    } else {
        quote! {}
    }
}

fn generate_final_state_branch(
    enum_name: &Ident,
    output: &Output,
    callback_def: TokenStream,
    is_skip: bool,
    state_ident: Ident,
    log_state: TokenStream,
) -> TokenStream {
    let jump = if callback_def.is_empty() {
        if is_skip {
            quote! { continue 'skip; }
        } else {
            let enumerator = ident!(output.value().0);
            quote! { return Some(Ok(#enum_name::#enumerator)); }
        }
    } else if is_skip {
        quote! {
            callback(lexer);
            continue 'skip;
        }
    } else {
        quote! { return Some(callback(lexer)); }
    };
    quote! {
        State::#state_ident => {
            #callback_def
            #log_state
            #jump
        }
    }
}

fn generate_state_branches<'a>(
    dfa: &'a Dfa,
    enum_name: &Ident,
    callbacks: HashMap<(String, usize), Expr>,
    luts: &mut BTreeMap<&'a Pattern, usize>,
) -> syn::Result<Vec<TokenStream>> {
    let mut branches = vec![];
    for (num, state) in dfa.states().iter().enumerate() {
        let state_ref = StateRef::new(num);
        let state_ident = ident!("S{num}");
        let log_state = crate::debug::log_state(num);
        let output = dfa.accepts().get(&state_ref).unwrap_or(&None);
        let is_skip = output
            .as_ref()
            .is_some_and(|output| output.value().0 == SKIP_NAME);
        let callback_def = generate_callback_def(enum_name, output, &callbacks, is_skip);

        branches.push(if !state.transitions().is_empty() {
            let last_accept = generate_last_accept(&callback_def, output, enum_name, is_skip);
            let transitions = generate_transitions(dfa, state_ref, state, luts);
            let loop_edge = if state.transitions().iter().any(|t| t.to() == state_ref) {
                quote! { loop }
            } else {
                quote! {}
            };
            quote! {
                State::#state_ident => {
                    #callback_def
                    #loop_edge {
                        #log_state
                        #last_accept
                        #transitions
                    }
                }
            }
        } else if let Some(output) = output {
            generate_final_state_branch(
                enum_name,
                output,
                callback_def,
                is_skip,
                state_ident,
                log_state,
            )
        } else {
            panic!("non-accepting state has no outgoing transitions, please report this bug")
        });
    }
    Ok(branches)
}

fn generate_stacked_lut_defs(luts: BTreeMap<&Pattern, usize>) -> Vec<TokenStream> {
    let mut lut_defs = vec![];
    let mut tables = vec![[0u8; 256]; (luts.len() + 7) / 8];
    for patterns in luts.into_iter().collect::<Vec<_>>().chunks(8) {
        for (p, i) in patterns.iter() {
            for b in u8::MIN..=u8::MAX {
                tables[i / 8][b as usize] |= if p.contains(b) { 1 << (i % 8) } else { 0 };
            }
        }
    }
    for (num, table) in tables.into_iter().enumerate() {
        let lut_ident = ident!("LUT{num}");
        lut_defs.push(quote! { const #lut_ident: [u8; 256] = [ #(#table),* ]; });
    }
    lut_defs
}

pub(crate) fn generate_impl(tokens: TokenStream) -> syn::Result<TokenStream> {
    let token_enum = parse_enum(tokens)?;
    let enum_name = token_enum.name;
    let enum_attrs = token_enum.attrs;
    let enum_variants = token_enum.variants;
    let dfa = generate_dfa(enum_variants.tokens)?;

    let states = (0..dfa.states().len())
        .map(|i| {
            let state = ident!("S{i}");
            quote! { #state }
        })
        .collect::<Vec<_>>();

    let mut luts = BTreeMap::new();
    let branches = generate_state_branches(&dfa, &enum_name, enum_variants.callbacks, &mut luts)?;

    let lut_defs = generate_stacked_lut_defs(luts);
    let ignore_call = enum_attrs.ignore_cb.map_or(quote! {}, |callback| {
        quote! {
            use herring::Source;
            let callback: fn(&mut herring::Lexer<'source, #enum_name>) = #callback;
            callback(lexer)
        }
    });
    let initial_call = enum_attrs.initial_cb.map_or(quote! {}, |callback| {
        quote! {
            let callback: fn(
                &mut herring::Lexer<'source, #enum_name>
            ) -> Option<Result<#enum_name, <Self as Herring<'source>>::Error>> = #callback;
            if let Some(tok) = callback(lexer) {
                return Some(tok);
            }
        }
    });
    let (error_ty, extras_ty, source_ty) = (
        enum_attrs.error_ty,
        enum_attrs.extras_ty,
        syn::Type::Verbatim(if enum_attrs.source_ty.is_empty() {
            if token_enum.binary {
                quote! { &'source [u8] }
            } else {
                quote! { &'source str }
            }
        } else {
            enum_attrs.source_ty
        }),
    );
    Ok(quote! {
        impl<'source> Herring<'source> for #enum_name {
            type Error = #error_ty;
            type Extras = #extras_ty;
            type Source = #source_ty;

            #[inline]
            fn ignore(lexer: &mut herring::Lexer<'source, #enum_name>) { #ignore_call }
            #[inline]
            fn lex(
                lexer: &mut herring::Lexer<'source, #enum_name>
            ) -> Option<Result<#enum_name, <Self as Herring<'source>>::Error>> {
                enum LastAccept<TokenCallback, SkipCallback> {
                    None,
                    Token(#enum_name, usize),
                    TokenCallback(TokenCallback, usize),
                    Skip(usize),
                    SkipCallback(SkipCallback, usize),
                }
                enum State {
                    #(#states,)*
                }
                #(#lut_defs)*

                'skip: loop {
                    lexer.start = lexer.offset;
                    #initial_call

                    let mut state = State::S0;
                    let mut last_accept: LastAccept<
                        fn(
                            &mut herring::Lexer<'source, #enum_name>
                        ) -> Result<#enum_name, <Self as Herring<'source>>::Error>,
                        fn(&mut herring::Lexer<'source, #enum_name>)
                    > = LastAccept::None;
                    'fsm: loop {
                        match state {
                            #(#branches)*
                        }
                    }
                    match last_accept {
                        LastAccept::None => {
                            use herring::Source;
                            while !lexer.source.is_boundary(lexer.offset) {
                                lexer.offset += 1;
                            }
                            return Some(Err(Default::default()));
                        }
                        LastAccept::Token(token, offset) => {
                            lexer.offset = offset;
                            return Some(Ok(token));
                        }
                        LastAccept::TokenCallback(callback, offset) => {
                            lexer.offset = offset;
                            return Some(callback(lexer));
                        }
                        LastAccept::Skip(offset) => {
                            lexer.offset = offset;
                        }
                        LastAccept::SkipCallback(callback, offset) => {
                            lexer.offset = offset;
                            callback(lexer);
                        }
                    }
                }
            }
        }
    })
}
