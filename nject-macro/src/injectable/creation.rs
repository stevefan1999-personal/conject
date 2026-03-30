use crate::attrs::InjectExpr;
use quote::quote;
use syn::{Ident, Type};

pub(crate) fn is_type_named(ty: &Type, name: &str) -> bool {
    if let Type::Path(type_path) = ty
        && let Some(seg) = type_path.path.segments.last()
    {
        seg.ident == name
    } else {
        false
    }
}

pub(crate) fn build_creation_output(
    ident: &Ident,
    types: &[&Type],
    keys: &[&Ident],
    attributes: &[Option<InjectExpr>],
) -> proc_macro2::TokenStream {
    if keys.is_empty() && !types.is_empty() {
        // Tuple struct
        let items = types
            .iter()
            .zip(attributes)
            .map(|(ty, a)| field_init_expr(ty, a));
        quote! { #ident(#(#items),*) }
    } else {
        // Named-field struct (or unit struct)
        let items = keys
            .iter()
            .zip(types.iter())
            .zip(attributes)
            .map(|((k, ty), a)| {
                let expr = field_init_expr(ty, a);
                quote! { #k: #expr }
            });
        quote! { #ident { #(#items),* } }
    }
}

pub(crate) fn field_init_expr(ty: &Type, attr: &Option<InjectExpr>) -> proc_macro2::TokenStream {
    match attr {
        Some(InjectExpr::Named(tag)) => {
            quote! { nject::Named::<#tag, #ty>::into_inner(provider.provide()) }
        }
        Some(InjectExpr::NamedStr(hash)) => {
            quote! { nject::Named::<nject::Key<#hash>, #ty>::into_inner(provider.provide()) }
        }
        Some(InjectExpr::Value(output, inputs)) => {
            let input_stmts = inputs
                .iter()
                .map(|x| quote! { let #x = provider.provide(); })
                .collect::<Vec<_>>();
            if input_stmts.is_empty() {
                quote! { #output }
            } else {
                quote! {
                    {
                        #(#input_stmts)*
                        #output
                    }
                }
            }
        }
        Some(InjectExpr::Env(key, default)) => {
            match default {
                Some(def) => quote! { std::env::var(#key).unwrap_or_else(|_| #def) },
                None => quote! { std::env::var(#key).expect(&format!("Missing env var: {}", #key)) },
            }
        }
        None if is_type_named(ty, "Option") => quote! { None },
        None if is_type_named(ty, "Late") => quote! { nject::Late::new() },
        None if is_type_named(ty, "Lazy") => quote! { nject::Lazy::new() },
        None => quote! { provider.provide() },
    }
}
