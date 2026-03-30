use crate::attrs::InjectExpr;
use crate::injectable::creation::is_type_named;
use quote::quote;
use syn::Type;

pub(crate) fn build_provider_bounds(
    types: &[&Type],
    attributes: &[Option<InjectExpr>],
) -> (Vec<proc_macro2::TokenStream>, proc_macro2::TokenStream) {
    let mut prov_types = Vec::<proc_macro2::TokenStream>::with_capacity(types.len());
    for (t, a) in types.iter().zip(attributes) {
        match a {
            Some(InjectExpr::Named(tag)) => {
                prov_types.push(quote! { ::conject::Named<#tag, #t> });
            }
            Some(InjectExpr::NamedStr(hash)) => {
                prov_types.push(quote! { ::conject::Named<::conject::Key<#hash>, #t> });
            }
            Some(InjectExpr::Value(_, inputs)) => {
                for attr_type in inputs.iter().map(|x| &x.ty) {
                    prov_types.push(quote! { #attr_type });
                }
            }
            #[cfg(feature = "env")]
            Some(InjectExpr::Env(..)) => {
                // Env injection is self-contained; no provider bounds needed.
            }
            None if !is_type_named(t, "Option")
                && !is_type_named(t, "Late")
                && !is_type_named(t, "Lazy") =>
            {
                prov_types.push(quote! { #t });
            }
            None => {}
        }
    }
    prov_types.dedup_by(|a, b| a.to_string() == b.to_string());
    let provider_bounds = if prov_types.is_empty() {
        quote! {}
    } else {
        quote! { ConjectProvider: #(::conject::Provider<'prov, #prov_types>)+*, }
    };
    (prov_types, provider_bounds)
}
