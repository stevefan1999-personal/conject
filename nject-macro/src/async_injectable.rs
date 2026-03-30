use crate::attrs::SimpleInjectExpr;
use crate::core::{DeriveInput, error};
use proc_macro::TokenStream;
use quote::quote;
use syn::spanned::Spanned;

pub(crate) fn handle_async_injectable(item: TokenStream) -> syn::Result<TokenStream> {
    let input = syn::parse::<DeriveInput>(item)?;
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
            attr.parse_args::<SimpleInjectExpr>()
                .map(Some)
                .map_err(|e| {
                    error::combine(
                        syn::Error::new(attr.span(), "Unable to parse inject attribute"),
                        e,
                    )
                })
        })
        .collect::<syn::Result<Vec<_>>>()?;
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
    let creation_output = if keys.is_empty() && !types.is_empty() {
        // Tuple struct (unnamed fields)
        let items = types.iter().zip(&attributes).map(|(t, a)| match a {
            Some(attr) => {
                let inputs = attr
                    .1
                    .iter()
                    .map(|x| {
                        let arg_ty = &x.ty;
                        quote! { let #x = <NjectProvider as nject::AsyncProvider<'prov, #arg_ty>>::provide(provider).await; }
                    })
                    .collect::<Vec<_>>();
                let output = &attr.0;
                if inputs.is_empty() {
                    quote! { #output }
                } else {
                    quote! {
                        {
                            #(#inputs)*
                            #output
                        }
                    }
                }
            }
            None => {
                quote! { <NjectProvider as nject::AsyncProvider<'prov, #t>>::provide(provider).await }
            }
        });
        quote! { #ident(#(#items),*) }
    } else {
        // Named fields struct
        let items = keys
            .iter()
            .zip(types.iter())
            .zip(&attributes)
            .map(|((k, t), a)| match a {
                Some(attr) => {
                    let inputs = attr
                        .1
                        .iter()
                        .map(|x| {
                            let arg_ty = &x.ty;
                            quote! { let #x = <NjectProvider as nject::AsyncProvider<'prov, #arg_ty>>::provide(provider).await; }
                        })
                        .collect::<Vec<_>>();
                    let output = &attr.0;
                    quote! {
                        #k: {
                            #(#inputs)*
                            #output
                        }
                    }
                }
                None => {
                    quote! { #k: <NjectProvider as nject::AsyncProvider<'prov, #t>>::provide(provider).await }
                }
            });
        quote! { #ident { #(#items),* } }
    };
    let mut prov_types = Vec::<_>::with_capacity(types.len());
    for (t, a) in types.iter().zip(&attributes) {
        if let Some(attr) = a {
            for attr_type in attr.1.iter().map(|x| &x.ty) {
                prov_types.push(quote! { #attr_type });
            }
        } else {
            prov_types.push(quote! { #t });
        }
    }
    prov_types.dedup_by(|a, b| a.to_string() == b.to_string());
    let output = quote! {
        #[derive(nject::InjectableHelperAttr)]
        #input

        impl<'prov, #(#generic_params,)*NjectProvider> nject::AsyncInjectable<'prov, #ident<#(#generic_keys),*>, NjectProvider> for #ident<#(#generic_keys),*>
            where
                #prov_lifetimes
                NjectProvider: #(nject::AsyncProvider<'prov, #prov_types>)+*, #where_predicates
        {
            #[inline]
            fn inject(provider: &'prov NjectProvider) -> impl core::future::Future<Output = #ident<#(#generic_keys),*>> {
                async move {
                    #creation_output
                }
            }
        }
    };
    Ok(output.into())
}
