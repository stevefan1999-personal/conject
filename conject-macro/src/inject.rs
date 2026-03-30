use crate::attrs::SimpleInjectExpr;
use crate::core::Generics;
use proc_macro::TokenStream;
use quote::quote;

pub(crate) fn handle_inject(item: TokenStream, attr: TokenStream) -> syn::Result<TokenStream> {
    let input = syn::parse::<syn::DeriveInput>(item)?;
    let attributes: SimpleInjectExpr = syn::parse(attr).map_err(|e| {
        let mut err = syn::Error::new(e.span(), "Unable to parse inject attribute.");
        err.combine(e);
        err
    })?;
    let ident = &input.ident;
    let Generics {
        params: generic_params,
        keys: generic_keys,
        prov_lifetimes,
        where_predicates,
    } = Generics::from_input(&input);
    let prov_types = attributes.1.iter().map(|x| &x.ty).collect::<Vec<_>>();
    let prov_input = attributes
        .1
        .iter()
        .map(|x| quote! { let #x = provider.provide(); })
        .collect::<Vec<_>>();
    let factory = attributes.0;
    let creation_output = quote! {
       #(#prov_input)*
       #factory
    };
    let output = quote! {
        #input

        impl<'prov, #(#generic_params,)*ConjectProvider> ::conject::Injectable<'prov, #ident<#(#generic_keys),*>, ConjectProvider> for #ident<#(#generic_keys),*>
            where
                #prov_lifetimes
                ConjectProvider: #(::conject::Provider<'prov, #prov_types>)+*, #where_predicates
        {
            #[inline(always)]
            fn inject(provider: &'prov ConjectProvider) -> #ident<#(#generic_keys),*> {
                #creation_output
            }
        }
    };
    Ok(output.into())
}
