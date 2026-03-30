use crate::attrs::InjectExpr;
use crate::core::DeriveInput;
use proc_macro::TokenStream;
use quote::quote;

#[allow(clippy::too_many_arguments)]
pub(super) fn handle_assisted_injectable(
    input: &DeriveInput,
    ident: &syn::Ident,
    fields: &syn::Fields,
    types: &[&syn::Type],
    keys: &[&syn::Ident],
    attributes: &[Option<InjectExpr>],
    assisted_flags: &[bool],
    generic_params: &[&syn::GenericParam],
    generic_keys: &[proc_macro2::TokenStream],
    _lifetime_keys: &[proc_macro2::TokenStream],
    prov_lifetimes: &proc_macro2::TokenStream,
    where_predicates: &proc_macro2::TokenStream,
) -> syn::Result<TokenStream> {
    let is_tuple = keys.is_empty() && !types.is_empty();

    // Collect assisted parameter definitions (name: Type) for the create fn signature
    let mut assisted_params = Vec::new();
    // Collect provider types (non-assisted, non-inject fields)
    let mut prov_types = Vec::new();

    for (i, (ty, attr)) in types.iter().zip(attributes.iter()).enumerate() {
        if assisted_flags[i] {
            // For named structs, use the field name; for tuple structs, generate a name
            let param_name = if is_tuple {
                syn::Ident::new(&format!("__assisted_{i}"), proc_macro2::Span::call_site())
            } else {
                keys.get(i)
                    .copied()
                    .expect("named field should have ident")
                    .clone()
            };
            assisted_params.push(quote! { #param_name: #ty });
        } else if let Some(InjectExpr::Value(_, inputs)) = attr {
            for attr_type in inputs.iter().map(|x| &x.ty) {
                prov_types.push(quote! { #attr_type });
            }
        } else if attr.is_none() {
            prov_types.push(quote! { #ty });
        }
    }
    prov_types.dedup_by(|a, b| a.to_string() == b.to_string());

    // Build the creation expression
    let creation_output = if is_tuple {
        let items = types
            .iter()
            .zip(attributes.iter())
            .zip(assisted_flags.iter())
            .enumerate()
            .map(|(i, ((_, attr), &is_assisted))| {
                if is_assisted {
                    let param_name = syn::Ident::new(
                        &format!("__assisted_{i}"),
                        proc_macro2::Span::call_site(),
                    );
                    quote! { #param_name }
                } else if let Some(InjectExpr::Value(output, inputs)) = attr {
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
                } else {
                    quote! { provider.provide() }
                }
            });
        quote! { #ident(#(#items),*) }
    } else {
        let items = fields.iter().enumerate().map(|(i, _f)| {
            let key = keys[i];
            if assisted_flags[i] {
                quote! { #key }
            } else if let Some(InjectExpr::Value(output, inputs)) = &attributes[i] {
                let input_stmts = inputs
                    .iter()
                    .map(|x| quote! { let #x = provider.provide(); })
                    .collect::<Vec<_>>();
                quote! {
                    #key: {
                        #(#input_stmts)*
                        #output
                    }
                }
            } else {
                quote! { #key: provider.provide() }
            }
        });
        quote! { #ident { #(#items),* } }
    };

    // Build the where clause for provider bounds
    let prov_bounds = if prov_types.is_empty() {
        quote! {}
    } else {
        quote! { NjectProvider: #(nject::Provider<'prov, #prov_types>)+*, }
    };

    let output = quote! {
        #[derive(nject::InjectableHelperAttr)]
        #input

        impl<#(#generic_params),*> #ident<#(#generic_keys),*>
            where #where_predicates
        {
            /// Create an instance with assisted injection.
            /// Non-assisted fields are resolved from the provider;
            /// assisted fields are passed as arguments.
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
