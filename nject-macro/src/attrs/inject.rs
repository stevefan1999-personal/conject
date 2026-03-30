use crate::core::FactoryExpr;
use darling::FromMeta;
use syn::{
    Expr, PatType, Token, Type,
    parse::{Parse, ParseStream},
};

#[derive(Clone)]
pub enum InjectExpr {
    Value(Box<Expr>, Vec<PatType>),
    Named(Type),
    NamedStr(u128),
    /// Environment variable injection: `env("KEY")` or `env("KEY", default_expr)`
    Env(String, Option<Box<Expr>>),
}

impl Parse for InjectExpr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(syn::Ident) {
            let fork = input.fork();
            if let Ok(ident) = fork.parse::<syn::Ident>() {
                if ident == "named" {
                    input.parse::<syn::Ident>()?;
                    let content;
                    syn::parenthesized!(content in input);
                    if content.peek(syn::LitStr) {
                        let lit: syn::LitStr = content.parse()?;
                        let hash = const_fnv1a_hash::fnv1a_hash_str_128(&lit.value());
                        return Ok(InjectExpr::NamedStr(hash));
                    }
                    return Ok(InjectExpr::Named(content.parse()?));
                }
                if ident == "env" {
                    input.parse::<syn::Ident>()?;
                    let content;
                    syn::parenthesized!(content in input);
                    let key: syn::LitStr = content.parse()?;
                    let default = if content.peek(Token![,]) {
                        content.parse::<Token![,]>()?;
                        Some(Box::new(content.parse::<Expr>()?))
                    } else {
                        None
                    };
                    return Ok(InjectExpr::Env(key.value(), default));
                }
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

impl FromMeta for InjectExpr {
    fn from_expr(expr: &Expr) -> darling::Result<Self> {
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

        if let Expr::Call(call) = expr
            && let Expr::Path(path) = &*call.func
            && path.path.is_ident("named")
            && call.args.len() == 1
        {
            let arg = &call.args[0];
            if let Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(lit_str),
                ..
            }) = arg
            {
                let hash = const_fnv1a_hash::fnv1a_hash_str_128(&lit_str.value());
                return Ok(InjectExpr::NamedStr(hash));
            }
            if let Expr::Path(type_path) = arg {
                return Ok(InjectExpr::Named(Type::Path(syn::TypePath {
                    qself: type_path.qself.clone(),
                    path: type_path.path.clone(),
                })));
            }
        }

        if let Expr::Call(call) = expr
            && let Expr::Path(path) = &*call.func
            && path.path.is_ident("env")
            && (call.args.len() == 1 || call.args.len() == 2)
        {
            if let Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(lit_str),
                ..
            }) = &call.args[0]
            {
                let default = if call.args.len() == 2 {
                    Some(Box::new(call.args[1].clone()))
                } else {
                    None
                };
                return Ok(InjectExpr::Env(lit_str.value(), default));
            }
        }

        Ok(InjectExpr::Value(Box::new(expr.clone()), vec![]))
    }
}

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
