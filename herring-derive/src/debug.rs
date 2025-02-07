use herring_automata::Automaton;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;

pub(crate) fn graph<const D: bool>(automaton: &Automaton<D>, name: &str) -> syn::Result<()> {
    if let Ok(val) = std::env::var("HERRING_DEBUG") {
        if val == "graphviz" {
            graphviz(automaton, &format!("{name}.dot"))
        } else if val == "mermaid" {
            mermaid(automaton, &format!("{name}.mmd"))
        } else {
            Ok(())
        }
    } else {
        Ok(())
    }
}

fn graphviz<const D: bool>(
    automaton: &herring_automata::Automaton<D>,
    name: &str,
) -> syn::Result<()> {
    use proc_macro2::Span;
    use syn::Error;
    if let Err(err) = automaton.print_graphviz(name) {
        return Err(Error::new(
            Span::call_site(),
            format!("error writing debug file: {err}"),
        ));
    }
    Ok(())
}

fn mermaid<const D: bool>(
    automaton: &herring_automata::Automaton<D>,
    name: &str,
) -> syn::Result<()> {
    use proc_macro2::Span;
    use syn::Error;
    if let Err(err) = automaton.print_mermaid(name) {
        return Err(Error::new(
            Span::call_site(),
            format!("error writing debug file: {err}"),
        ));
    }
    Ok(())
}

pub(crate) fn log_state(state: usize) -> TokenStream {
    if let Ok(val) = std::env::var("HERRING_DEBUG") {
        if val == "log" {
            return quote! { eprintln!("STATE: S{}", #state); };
        }
    }
    quote! {}
}

pub(crate) fn expand_or_skip(output: TokenStream, enum_name: &Ident) -> syn::Result<TokenStream> {
    fn write_file(output: TokenStream, enum_name: &Ident) -> std::io::Result<()> {
        use std::io::Write;
        let mut f =
            std::fs::File::create(format!("{}_lexer.rs", enum_name.to_string().to_lowercase()))?;
        write!(f, "{output}")
    }
    if let Ok(val) = std::env::var("HERRING_DEBUG") {
        match val.as_str() {
            "expand" => {
                if let Err(err) = write_file(output, enum_name) {
                    return Err(syn::Error::new(
                        Span::call_site(),
                        format!("error writing output file: {err}"),
                    ));
                }
                return Ok(quote! {});
            }
            "skip" => {
                return Ok(quote! {});
            }
            _ => {}
        }
    }
    Ok(output)
}
