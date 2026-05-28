mod assisted;
pub(crate) mod creation;
mod provider_bounds;

use crate::attrs::{InjectableAttrs, ParsedField};
use crate::core::{DeriveInputExt, Generics};
use proc_macro::TokenStream;
use quote::quote;

/// If `expr` is a closure whose first parameter is an untyped `Pat::Ident`,
/// rewrite it to `Pat::Type` with `&mut #self_ty` so the compiler can infer
/// async-block lifetimes correctly.
fn annotate_closure_first_param(expr: &syn::Expr, self_ty: &syn::Type) -> syn::Expr {
    let syn::Expr::Closure(closure) = expr else {
        return expr.clone();
    };
    let mut closure = closure.clone();
    if let Some(first) = closure.inputs.first_mut() {
        if let syn::Pat::Ident(pat_ident) = first {
            *first = syn::Pat::Type(syn::PatType {
                attrs: vec![],
                pat: Box::new(syn::Pat::Ident(pat_ident.clone())),
                colon_token: Default::default(),
                ty: Box::new(syn::parse_quote!(&mut #self_ty)),
            });
        }
    }
    syn::Expr::Closure(closure)
}

pub(crate) fn handle_injectable(item: TokenStream) -> syn::Result<TokenStream> {
    let mut input = syn::parse::<syn::DeriveInput>(item)?;

    let injectable_attrs: InjectableAttrs =
        InjectableAttrs::from_attrs(&input.attrs).map_err(syn::Error::from)?;
    InjectableAttrs::strip_from(&mut input.attrs);

    let ident = &input.ident;
    let fields = input.fields();
    let types = input.field_types();
    let keys = input.field_idents();

    let parsed_fields = ParsedField::from_fields(fields.iter()).map_err(syn::Error::from)?;

    let attributes: Vec<Option<crate::attrs::InjectExpr>> =
        parsed_fields.iter().map(|pf| pf.inject.clone()).collect();
    let assisted_flags: Vec<bool> = parsed_fields.iter().map(|pf| pf.assisted).collect();
    let has_assisted = assisted_flags.iter().any(|&x| x);

    let g = Generics::from_input(&input);

    if has_assisted {
        return assisted::handle_assisted_injectable(
            &input,
            ident,
            fields,
            &types,
            &keys,
            &attributes,
            &assisted_flags,
            &g,
        );
    }

    let creation_output = creation::build_creation_output(ident, &types, &keys, &attributes);
    let creation_output = match &injectable_attrs.post_construct {
        Some(expr) => quote! { (#expr)(#creation_output) },
        None => creation_output,
    };
    let (prov_types, provider_bounds) = provider_bounds::build_provider_bounds(&types, &attributes);
    let Generics {
        params: generic_params,
        keys: generic_keys,
        prov_lifetimes,
        where_predicates,
    } = &g;
    let pre_destroy_output = match &injectable_attrs.pre_destroy {
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
    let async_pre_destroy_output = match &injectable_attrs.async_pre_destroy {
        Some(expr) => {
            // If the expression is a closure with an untyped first parameter,
            // annotate it with `&mut Self` so the compiler can infer the async
            // block's lifetime (without this, `|s| async move { s.field }` fails).
            let expr =
                annotate_closure_first_param(expr, &syn::parse_quote!(#ident<#(#generic_keys),*>));
            quote! {
                impl<#(#generic_params),*> #ident<#(#generic_keys),*>
                where #where_predicates
                {
                    /// Async cleanup. Call before dropping.
                    pub async fn destroy(&mut self) {
                        (#expr)(self).await;
                    }
                }
            }
        }
        None => quote! {},
    };
    let output = quote! {
        #[derive(::conject::InjectableHelperAttr)]
        #input

        impl<'prov, #(#generic_params,)*ConjectProvider> ::conject::Injectable<'prov, #ident<#(#generic_keys),*>, ConjectProvider> for #ident<#(#generic_keys),*>
            where
                #prov_lifetimes
                #provider_bounds #where_predicates
        {
            #[inline(always)]
            fn inject(provider: &'prov ConjectProvider) -> #ident<#(#generic_keys),*> {
                #creation_output
            }
        }

        impl<'prov, #(#generic_params,)*ConjectProvider> ::conject::AsyncInjectable<'prov, #ident<#(#generic_keys),*>, ConjectProvider> for #ident<#(#generic_keys),*>
            where
                #prov_lifetimes
                ConjectProvider: #(::conject::Provider<'prov, #prov_types>)+*, #where_predicates
        {
            #[inline(always)]
            fn inject(provider: &'prov ConjectProvider) -> impl ::core::future::Future<Output = #ident<#(#generic_keys),*>> {
                ::core::future::ready(
                    <Self as ::conject::Injectable<'prov, #ident<#(#generic_keys),*>, ConjectProvider>>::inject(provider)
                )
            }
        }

        #pre_destroy_output

        #async_pre_destroy_output
    };
    Ok(output.into())
}
