use crate::attrs::InjectExpr;
use quote::quote;
use syn::{Ident, Type};

/// Check if a type's last path segment matches the given name (e.g. "Option", "Late", "Lazy").
pub(crate) fn is_type_named(ty: &Type, name: &str) -> bool {
    if let Type::Path(type_path) = ty
        && let Some(seg) = type_path.path.segments.last()
    {
        seg.ident == name
    } else {
        false
    }
}

/// Build the struct construction expression for `Injectable::inject`.
///
/// Handles both tuple structs (unnamed fields) and named-field structs.
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
        let items =
            keys.iter()
                .zip(types.iter())
                .zip(attributes)
                .map(|((k, ty), a)| match a {
                    Some(InjectExpr::Named(tag)) => {
                        quote! {
                            #k: nject::Named::<#tag, #ty>::into_inner(provider.provide())
                        }
                    }
                    Some(InjectExpr::NamedStr(hash)) => {
                        quote! {
                            #k: nject::Named::<nject::Key<#hash>, #ty>::into_inner(provider.provide())
                        }
                    }
                    Some(InjectExpr::Value(output, inputs)) => {
                        let input_stmts = inputs
                            .iter()
                            .map(|x| quote! { let #x = provider.provide(); })
                            .collect::<Vec<_>>();
                        quote! {
                            #k: {
                                #(#input_stmts)*
                                #output
                            }
                        }
                    }
                    None if is_type_named(ty, "Option") => quote! { #k: None },
                    None if is_type_named(ty, "Late") => quote! { #k: nject::Late::new() },
                    None if is_type_named(ty, "Lazy") => quote! { #k: nject::Lazy::new() },
                    None => quote! { #k: provider.provide() },
                });
        quote! { #ident { #(#items),* } }
    }
}

/// Generate the initialization expression for a single field.
fn field_init_expr(ty: &Type, attr: &Option<InjectExpr>) -> proc_macro2::TokenStream {
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
        None if is_type_named(ty, "Option") => quote! { None },
        None if is_type_named(ty, "Late") => quote! { nject::Late::new() },
        None if is_type_named(ty, "Lazy") => quote! { nject::Lazy::new() },
        None => quote! { provider.provide() },
    }
}
