use crate::attrs::ProvideFieldInput;
use quote::quote;
use syn::{GenericParam, Ident, Type};

pub(super) fn gen_providers_for_provide_attr_on_fields(
    ident: &Ident,
    generic_params: &[&GenericParam],
    generic_keys: &[proc_macro2::TokenStream],
    where_predicates: &proc_macro2::TokenStream,
    fields_path_prefix: &proc_macro2::TokenStream,
    fields: &[&syn::Field],
    provide_attr_indexes: &[(usize, Vec<&syn::Attribute>)],
) -> Vec<proc_macro2::TokenStream> {
    let provide_outputs = provide_attr_indexes.iter().map(|(i, attrs)| {
        let field = fields[*i];
        let (key_prefix, ref_prefix) = if let Type::Reference(r) = &field.ty {
            let lifetime = &r.lifetime;
            (quote! {}, quote! { &#lifetime })
        } else {
            (quote! { & }, quote! { &'prov })
        };
        let inputs = attrs.iter().map(|a| match a.meta {
            syn::Meta::Path(_) => ProvideFieldInput::Type(field.ty.to_owned()),
            _ => a.parse_args::<ProvideFieldInput>().unwrap()
        });
        let index = syn::Index::from(*i);
        let field_key = match &field.ident {
            Some(i) => quote! { #i },
            None => quote! { #index },
        };
        let outputs = inputs.map(|input| {
            let ty = match &input {
                ProvideFieldInput::None => match &field.ty {
                    Type::Reference(r) => {
                        let inner_ty = &r.elem;
                        quote! { #ref_prefix #inner_ty }
                    },
                    _ => {
                        let ty = &field.ty;
                        quote! { #ref_prefix #ty }
                    },
                },
                ProvideFieldInput::Type(t) => match t {
                    Type::Reference(r) => {
                        let inner_ty = &r.elem;
                        quote! { #ref_prefix #inner_ty }
                    },
                    _ => quote! { #ref_prefix #t },
                },
                ProvideFieldInput::TypeExpr(t, _, _) => quote! { #t },
            };
            let body = match &input {
                ProvideFieldInput::TypeExpr(_, i, e) => {
                    let ref_prefix = match &field.ty {
                        Type::Reference(_) => quote!{},
                        _ => quote!{&},
                    };
                    quote!{
                        let #i = #ref_prefix self.#fields_path_prefix #field_key;
                        #e
                    }
                },
                _ => quote!{ #key_prefix self.#fields_path_prefix #field_key }
            };

            quote! {

                impl<'prov, #(#generic_params),*> nject::Provider<'prov, #ty> for #ident<#(#generic_keys),*>
                    where #where_predicates
                {
                    #[inline]
                    fn provide(&'prov self) -> #ty {
                       #body
                    }
                }
            }
        });

        quote! {
            #(#outputs)*
        }
    });
    provide_outputs.collect()
}
