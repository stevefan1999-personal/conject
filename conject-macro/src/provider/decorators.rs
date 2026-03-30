use std::collections::HashMap;

use crate::attrs::{DecorateStructInput, ProvideStructInput};
use quote::quote;
use syn::{GenericParam, Ident};

pub(crate) fn gen_providers_for_provide_attr_on_struct(
    ident: &Ident,
    generic_params: &[&GenericParam],
    generic_keys: &[proc_macro2::TokenStream],
    where_predicates: &proc_macro2::TokenStream,
    provide_input_attr: &[&syn::Attribute],
    decorate_input_attr: &[&syn::Attribute],
) -> Vec<proc_macro2::TokenStream> {
    let parsed_decorators: Vec<DecorateStructInput> = decorate_input_attr
        .iter()
        .map(|a| a.parse_args::<DecorateStructInput>().unwrap())
        .collect();
    let mut decor_by_type = HashMap::<String, Vec<&DecorateStructInput>>::new();
    for d in &parsed_decorators {
        let ty = &d.ty;
        let key = quote! { #ty }.to_string();
        decor_by_type.entry(key).or_default().push(d);
    }

    let input_provide_outputs = provide_input_attr
        .iter()
        .map(|a| a.parse_args::<ProvideStructInput>().unwrap())
        .map(|t| {
            let (ty, inputs, value) = match t {
                ProvideStructInput::TypeExpr(t, v) => (t, vec![], v),
                ProvideStructInput::TypeExprFact(t, i, v) => (t, i, v),
            };
            let type_key = quote! { #ty }.to_string();
            let decorators = decor_by_type.get(&type_key);
            let body = if let Some(decorators) = decorators {
                let decorator_steps = decorators.iter().map(|d| {
                    let var = &d.var;
                    let expr = &d.expr;
                    quote! {
                        let __conject_inner = {
                            let #var = __conject_inner;
                            #expr
                        };
                    }
                });
                quote! {
                    #(let #inputs = self.provide();)*
                    let __conject_inner = { #value };
                    #(#decorator_steps)*
                    __conject_inner
                }
            } else {
                quote! {
                    #(let #inputs = self.provide();)*
                    #value
                }
            };
            quote!{

                impl<'prov, #(#generic_params),*> ::conject::Provider<'prov, #ty> for #ident<#(#generic_keys),*>
                    where #where_predicates
                {
                    #[inline(always)]
                    fn provide(&'prov self) -> #ty {
                        #body
                    }
                }
            }
        });
    input_provide_outputs.collect()
}
