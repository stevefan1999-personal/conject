use crate::core::FactoryExpr;
use darling::FromMeta;
use syn::{
    Expr, PatType, Token, Type,
    parse::{Parse, ParseStream},
};

/// Parsed representation of the `#[inject(...)]` attribute on a field.
///
/// Supports three forms:
/// - `#[inject(expr)]` or `#[inject(|dep: T| expr)]` — direct value / factory
/// - `#[inject(named(TagType))]` — named injection by tag type
/// - `#[inject(named("key"))]` — named injection by string key (FNV-hashed)
#[derive(Clone)]
pub enum InjectExpr {
    /// A direct expression, optionally with factory inputs: `expr` or `|dep: T| expr`
    Value(Box<Expr>, Vec<PatType>),
    /// A named injection tag type: `named(TagType)`
    Named(Type),
    /// A named injection string key: `named("key")` -- resolved via `Key<{hash}>`
    NamedStr(u128),
}

impl Parse for InjectExpr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Check for `named(TagType)` or `named("string_key")` syntax
        if input.peek(syn::Ident) {
            let fork = input.fork();
            if let Ok(ident) = fork.parse::<syn::Ident>()
                && ident == "named"
            {
                // Commit to the `named(...)` parse
                input.parse::<syn::Ident>()?; // consume "named"
                let content;
                syn::parenthesized!(content in input);
                // Check for string literal: named("key")
                if content.peek(syn::LitStr) {
                    let lit: syn::LitStr = content.parse()?;
                    let hash_bytes = crate::core::hash::fnv(lit.value().as_bytes());
                    let hash = u128::from_be_bytes(hash_bytes);
                    return Ok(InjectExpr::NamedStr(hash));
                }
                // Otherwise parse as type: named(TagType)
                let tag_type: Type = content.parse()?;
                return Ok(InjectExpr::Named(tag_type));
            }
        }
        if input.peek(Token![|]) {
            let expr = FactoryExpr::parse(input)?;
            Ok(InjectExpr::Value(expr.body, expr.inputs))
        } else {
            Ok(InjectExpr::Value(input.parse()?, vec![]))
        }
    }
}

/// darling integration: allows `#[inject(expr)]` to be parsed via darling's `FromMeta`.
///
/// Uses `from_expr` for valid Rust expressions (covers `42`, `MyStruct { v: 1 }`,
/// `|dep: T| expr`, `named(Tag)`, etc.) and `from_list` for parenthesized token streams
/// that darling can't parse as expressions.
impl FromMeta for InjectExpr {
    fn from_expr(expr: &Expr) -> darling::Result<Self> {
        // Closures: |dep: T| expr
        if let Expr::Closure(closure) = expr {
            let mut inputs = Vec::with_capacity(closure.inputs.len());
            for input in &closure.inputs {
                if let syn::Pat::Type(pat_type) = input {
                    inputs.push(pat_type.clone());
                } else {
                    return Err(darling::Error::custom(format!(
                        "Invalid closure input: {}",
                        quote::quote! { #input }
                    )));
                }
            }
            return Ok(InjectExpr::Value(closure.body.clone(), inputs));
        }

        // named(Tag) or named("key") — appears as a function call expression
        if let Expr::Call(call) = expr {
            if let Expr::Path(path) = &*call.func {
                if path.path.is_ident("named") && call.args.len() == 1 {
                    let arg = &call.args[0];
                    // named("key") — string literal
                    if let Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(lit_str),
                        ..
                    }) = arg
                    {
                        let hash_bytes = crate::core::hash::fnv(lit_str.value().as_bytes());
                        let hash = u128::from_be_bytes(hash_bytes);
                        return Ok(InjectExpr::NamedStr(hash));
                    }
                    // named(TagType) — parse the arg as a type path
                    if let Expr::Path(type_path) = arg {
                        let ty: Type = Type::Path(syn::TypePath {
                            qself: type_path.qself.clone(),
                            path: type_path.path.clone(),
                        });
                        return Ok(InjectExpr::Named(ty));
                    }
                }
            }
        }

        // Everything else: direct expression
        Ok(InjectExpr::Value(Box::new(expr.clone()), vec![]))
    }
}

/// Simpler `#[inject(...)]` for `#[inject]` / `#[async_injectable]` struct-level attribute
/// and the `inject.rs` handler. Only supports `expr` or `|dep: T| expr`.
pub struct SimpleInjectExpr(pub Box<Expr>, pub Vec<PatType>);

impl Parse for SimpleInjectExpr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(Token![|]) {
            let expr = FactoryExpr::parse(input)?;
            Ok(SimpleInjectExpr(expr.body, expr.inputs))
        } else {
            Ok(SimpleInjectExpr(input.parse()?, vec![]))
        }
    }
}
