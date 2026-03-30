mod assisted;
pub(crate) mod creation;
mod provider_bounds;

use crate::attrs::{InjectableAttrs, ParsedField};
use crate::core::DeriveInput;
use proc_macro::TokenStream;
use quote::quote;

pub(crate) fn handle_injectable(item: TokenStream) -> syn::Result<TokenStream> {
    let mut input = syn::parse::<DeriveInput>(item)?;

    // Use centralized struct-level attribute parsing via darling error accumulation
    let injectable_attrs: InjectableAttrs = InjectableAttrs::from_attrs(&input.0.attrs)
        .map_err(syn::Error::from)?;
    InjectableAttrs::strip_from(&mut input.0.attrs);

    let ident = &input.ident;
    let fields = input.fields();
    let types = input.field_types();
    let keys = input.field_idents();

    // Use centralized field parsing with darling error accumulation
    let parsed_fields = ParsedField::from_fields(fields.iter())
        .map_err(syn::Error::from)?;

    let attributes: Vec<Option<crate::attrs::InjectExpr>> =
        parsed_fields.iter().map(|pf| pf.inject.clone()).collect();
    let assisted_flags: Vec<bool> = parsed_fields.iter().map(|pf| pf.assisted).collect();
    let has_assisted = assisted_flags.iter().any(|&x| x);

    // Validation (inject + assisted) is already done inside ParsedField::from_field

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
    let creation_output = match &injectable_attrs.post_construct {
        Some(expr) => quote! { (#expr)(#creation_output) },
        None => creation_output,
    };
    let (prov_types, provider_bounds) =
        provider_bounds::build_provider_bounds(&types, &attributes);
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
