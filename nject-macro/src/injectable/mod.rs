mod assisted;
mod creation;
mod provider_bounds;

use crate::attrs::InjectExpr;
use crate::core::{DeriveInput, error};
use proc_macro::TokenStream;
use quote::quote;
use syn::{Expr, spanned::Spanned};

pub(crate) fn handle_injectable(item: TokenStream) -> syn::Result<TokenStream> {
    let mut input = syn::parse::<DeriveInput>(item)?;
    let post_construct = input
        .0
        .attrs
        .iter()
        .find(|a| a.path().is_ident("post_construct"))
        .map(|a| a.parse_args::<Expr>())
        .transpose()?;
    let pre_destroy = input
        .0
        .attrs
        .iter()
        .find(|a| a.path().is_ident("pre_destroy"))
        .map(|a| a.parse_args::<Expr>())
        .transpose()?;
    input
        .0
        .attrs
        .retain(|a| !a.path().is_ident("post_construct") && !a.path().is_ident("pre_destroy"));
    let ident = &input.ident;
    let fields = input.fields();
    let types = input.field_types();
    let keys = input.field_idents();
    let attributes = fields
        .iter()
        .map(|f| {
            let Some(attr) = f.attrs.iter().rfind(|a| a.path().is_ident("inject")) else {
                return Ok(None);
            };
            attr.parse_args::<InjectExpr>().map(Some).map_err(|e| {
                error::combine(
                    syn::Error::new(attr.span(), "Unable to parse inject attribute"),
                    e,
                )
            })
        })
        .collect::<syn::Result<Vec<_>>>()?;

    // Detect #[assisted] fields
    let assisted_flags: Vec<bool> = fields
        .iter()
        .map(|f| f.attrs.iter().any(|a| a.path().is_ident("assisted")))
        .collect();
    let has_assisted = assisted_flags.iter().any(|&x| x);

    // Validate: a field cannot be both #[inject] and #[assisted]
    for (i, f) in fields.iter().enumerate() {
        if assisted_flags[i] && attributes[i].is_some() {
            return Err(syn::Error::new(
                f.attrs
                    .iter()
                    .find(|a| a.path().is_ident("assisted"))
                    .unwrap()
                    .span(),
                "A field cannot have both #[inject] and #[assisted] attributes",
            ));
        }
    }

    let generic_params = input.generic_params();
    let generic_keys = input.generic_keys();
    let lifetime_keys = input.lifetime_keys();
    let prov_lifetimes = if lifetime_keys.is_empty() {
        quote! {}
    } else {
        quote! { 'prov: #(#lifetime_keys)+*, }
    };
    let where_predicates = match &input.generics.where_clause {
        Some(w) => {
            let predicates = &w.predicates;
            quote! { #predicates }
        }
        None => quote! {},
    };

    if has_assisted {
        return assisted::handle_assisted_injectable(
            &input,
            ident,
            fields,
            &types,
            &keys,
            &attributes,
            &assisted_flags,
            &generic_params,
            &generic_keys,
            &lifetime_keys,
            &prov_lifetimes,
            &where_predicates,
        );
    }

    let creation_output =
        creation::build_creation_output(ident, &types, &keys, &attributes);
    let creation_output = match &post_construct {
        Some(expr) => quote! { (#expr)(#creation_output) },
        None => creation_output,
    };
    let (prov_types, provider_bounds) =
        provider_bounds::build_provider_bounds(&types, &attributes);
    let pre_destroy_output = match &pre_destroy {
        Some(expr) => quote! {
            impl<#(#generic_params),*> Drop for #ident<#(#generic_keys),*>
            where #where_predicates
            {
                fn drop(&mut self) {
                    (#expr)(self);
                }
            }
        },
        None => quote! {},
    };
    let output = quote! {
        #[derive(nject::InjectableHelperAttr)]
        #input

        impl<'prov, #(#generic_params,)*NjectProvider> nject::Injectable<'prov, #ident<#(#generic_keys),*>, NjectProvider> for #ident<#(#generic_keys),*>
            where
                #prov_lifetimes
                #provider_bounds #where_predicates
        {
            #[inline]
            fn inject(provider: &'prov NjectProvider) -> #ident<#(#generic_keys),*> {
                #creation_output
            }
        }

        impl<'prov, #(#generic_params,)*NjectProvider> nject::AsyncInjectable<'prov, #ident<#(#generic_keys),*>, NjectProvider> for #ident<#(#generic_keys),*>
            where
                #prov_lifetimes
                NjectProvider: #(nject::Provider<'prov, #prov_types>)+*, #where_predicates
        {
            #[inline]
            fn inject(provider: &'prov NjectProvider) -> impl core::future::Future<Output = #ident<#(#generic_keys),*>> {
                core::future::ready(
                    <Self as nject::Injectable<'prov, #ident<#(#generic_keys),*>, NjectProvider>>::inject(provider)
                )
            }
        }

        #pre_destroy_output
    };
    Ok(output.into())
}
