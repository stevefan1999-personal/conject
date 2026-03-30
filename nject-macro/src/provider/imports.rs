use itertools::Itertools;
use quote::quote;
use syn::{GenericParam, Ident, Type};

pub(super) fn gen_imports_for_import_attr(
    ident: &Ident,
    generic_params: &[&GenericParam],
    generic_keys: &[proc_macro2::TokenStream],
    where_predicates: &proc_macro2::TokenStream,
    fields_path_prefix: &proc_macro2::TokenStream,
    fields: &[&syn::Field],
    import_attr_indexes: &[usize],
) -> Vec<proc_macro2::TokenStream> {
    let imported_modules = import_attr_indexes
        .iter()
        .map(|i| {
            let field = fields[*i];
            let ty = &field.ty;
            let import_key = crate::module::models::ModuleKey::from(ty);
            let import = crate::module::repository::get(&import_key);
            let index = syn::Index::from(*i);
            let field_key = match &field.ident {
                Some(i) => quote! { #i },
                None => quote! { #index },
            };
            (import, field_key, field)
        })
        .collect::<Vec<_>>();
    let exported_types = imported_modules.iter().flat_map(|(module, field_key, _)| {
        module
            .as_ref()
            .map(|m| {
                m.exported_types()
                    .iter()
                    .map(|t| (m.to_owned(), t.to_owned()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
            .iter()
            .map(|(m, t)| (m.to_owned(), t.to_owned(), field_key))
            .collect::<Vec<_>>()
    });
    let exported_types = Itertools::into_group_map_by(exported_types, |(_, t, _)| quote! { #t }.to_string());
    let import_iter_outputs = exported_types.values().map(|types| {
        let (_, ty, _) = types.first().unwrap();
        let types_by_module = Itertools::into_group_map_by(types.iter().enumerate(), |(_, (m, _, _))| {
            m.key().unwrap()
        });
        let iter_match_outputs = types_by_module.values().flat_map(|types_for_module| {
            types_for_module.iter().enumerate().map(|(mod_index, (index, (_, ty, field_key)))| {
                quote! { #index => nject::RefIterable::<#ty, #ident<#(#generic_keys),*>>::inject(&self.provider.#fields_path_prefix #field_key, self.provider, #mod_index), }
            })
        });
        quote!{

            impl<'prov, #(#generic_params),*> nject::Iterable<'prov, #ty> for #ident<#(#generic_keys),*>
                where #where_predicates
            {
                #[inline]
                fn iter(&'prov self) -> impl Iterator<Item = #ty> {
                    struct NjectIterator<'prov, #(#generic_params),*> {
                        provider: &'prov #ident<#(#generic_keys),*>,
                        index: usize,
                    }
                    impl<'prov, #(#generic_params),*> Iterator for NjectIterator<'prov, #(#generic_keys),*> {
                        type Item = #ty;

                        fn next(&mut self) -> Option<Self::Item> {
                            let result = match self.index {
                                #( #iter_match_outputs )*
                                _ => {
                                    return None;
                                }
                            };
                            self.index += 1;
                            Some(result)
                        }
                    }
                    NjectIterator {
                        provider: self,
                        index: 0,
                    }
                }
            }
        }
    });
    let import_prov_outputs = exported_types.values().map(|types| {
        let (_, ty, field_key) = types.last().unwrap();
        quote!{

            impl<'prov, #(#generic_params),*> nject::Provider<'prov, #ty> for #ident<#(#generic_keys),*>
                where #where_predicates
            {
                #[inline]
                fn provide(&'prov self) -> #ty {
                    nject::RefInjectable::<#ty, Self>::inject(&self.#fields_path_prefix #field_key, self)
                }
            }
        }
    });
    let import_impl_outputs = imported_modules.iter().map(|(_, field_key, field)| {
        let ty = &field.ty;
        let ty_output = if let Type::Reference(r) = ty {
            let inner_ty = &r.elem;
            quote! { #inner_ty }
        } else {
            quote! { #ty }
        };
        quote! {

            impl<#(#generic_params),*> nject::Import<#ty_output> for #ident<#(#generic_keys),*>
                where #where_predicates
            {
                #[inline]
                fn reference(&self) -> & #ty_output {
                    &self.#fields_path_prefix #field_key
                }
            }
        }
    });
    import_impl_outputs
        .chain(import_prov_outputs)
        .chain(import_iter_outputs)
        .collect()
}
