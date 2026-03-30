use crate::attrs::InjectExpr;
use crate::core::Generics;
use proc_macro::TokenStream;
use quote::quote;

#[allow(clippy::too_many_arguments)]
pub(super) fn handle_assisted_injectable(
    input: &syn::DeriveInput,
    ident: &syn::Ident,
    fields: &syn::Fields,
    types: &[&syn::Type],
    keys: &[&syn::Ident],
    attributes: &[Option<InjectExpr>],
    assisted_flags: &[bool],
    g: &Generics<'_>,
) -> syn::Result<TokenStream> {
    let Generics {
        params: generic_params,
        keys: generic_keys,
        prov_lifetimes,
        where_predicates,
    } = g;
    let is_tuple = keys.is_empty() && !types.is_empty();

    let mut assisted_params = Vec::new();

    for (i, ty) in types.iter().enumerate() {
        if assisted_flags[i] {
            let param_name = if is_tuple {
                syn::Ident::new(&format!("__assisted_{i}"), proc_macro2::Span::call_site())
            } else {
                keys.get(i)
                    .copied()
                    .expect("named field should have ident")
                    .clone()
            };
            assisted_params.push(quote! { #param_name: #ty });
        }
    }

    let non_assisted: Vec<_> = types
        .iter()
        .zip(attributes.iter())
        .zip(assisted_flags.iter())
        .filter(|(_, assisted)| !**assisted)
        .map(|((ty, attr), _)| (*ty, attr.clone()))
        .collect();
    let na_types: Vec<_> = non_assisted.iter().map(|(ty, _)| *ty).collect();
    let na_attrs: Vec<_> = non_assisted.iter().map(|(_, attr)| attr.clone()).collect();
    let (_, prov_bounds) = super::provider_bounds::build_provider_bounds(&na_types, &na_attrs);

    let creation_output = if is_tuple {
        let items = types
            .iter()
            .zip(attributes.iter())
            .zip(assisted_flags.iter())
            .enumerate()
            .map(|(i, ((ty, attr), &is_assisted))| {
                if is_assisted {
                    let param_name =
                        syn::Ident::new(&format!("__assisted_{i}"), proc_macro2::Span::call_site());
                    quote! { #param_name }
                } else {
                    super::creation::field_init_expr(ty, attr)
                }
            });
        quote! { #ident(#(#items),*) }
    } else {
        let items = fields.iter().enumerate().map(|(i, _f)| {
            let key = keys[i];
            if assisted_flags[i] {
                quote! { #key }
            } else {
                let expr = super::creation::field_init_expr(types[i], &attributes[i]);
                quote! { #key: #expr }
            }
        });
        quote! { #ident { #(#items),* } }
    };

    let output = quote! {
        #[derive(nject::InjectableHelperAttr)]
        #input

        impl<#(#generic_params),*> #ident<#(#generic_keys),*>
            where #where_predicates
        {
            #[inline]
            pub fn create<'prov, NjectProvider>(
                provider: &'prov NjectProvider,
                #(#assisted_params),*
            ) -> Self
            where
                #prov_lifetimes
                #prov_bounds
            {
                #creation_output
            }
        }
    };
    Ok(output.into())
}
