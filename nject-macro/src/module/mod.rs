pub mod models;
pub mod repository;
use crate::attrs::{ExportFieldInput, ExportStructInput, ParsedField};
use crate::core::{DeriveInputExt, Generics};
use itertools::Itertools;
use darling::Error as DarlingError;
use proc_macro::TokenStream;
use quote::quote;
use syn::{Path, Type};

pub(crate) fn handle_module(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let input = syn::parse::<syn::DeriveInput>(item)?;
    let module_pub_path = if attr.is_empty() {
        None
    } else {
        let path = syn::parse::<Path>(attr).map_err(|e| {
            let mut err = syn::Error::new(e.span(), "Invalid public module path");
            err.combine(e);
            err
        })?;
        Some(path)
    };
    let ident = &input.ident;
    let fields = input.fields().iter().collect::<Vec<_>>();

    let struct_exports = {
        let mut errors = DarlingError::accumulator();
        let mut exports = Vec::new();
        for attr in input.attrs.iter().filter(|a| a.path().is_ident("export")) {
            match attr.parse_args::<ExportStructInput>() {
                Ok(e) => exports.push(e),
                Err(e) => errors.push(
                    DarlingError::custom(format!("Unable to parse struct export attribute: {e}"))
                        .with_span(attr),
                ),
            }
        }
        errors.finish().map_err(syn::Error::from)?;
        exports
    };

    let parsed_fields = ParsedField::from_fields(input.fields().iter())
        .map_err(syn::Error::from)?;

    let export_attr_indexes: Vec<(usize, Vec<&syn::Attribute>)> = parsed_fields
        .iter()
        .enumerate()
        .filter_map(|(i, pf)| {
            if pf.export_attrs.is_empty() {
                return None;
            }
            let attrs = fields[i]
                .attrs
                .iter()
                .filter(|a| a.path().is_ident("export"))
                .collect::<Vec<_>>();
            Some((i, attrs))
        })
        .collect();
    let struct_exports_by_type = struct_exports.iter().into_group_map_by(|k| match k {
        ExportStructInput::TypeExpr(t, _) => quote! { #t }.to_string(),
        ExportStructInput::TypeExprFact(t, _, _) => quote! { #t }.to_string(),
    });
    let struct_type_exports = struct_exports
        .iter()
        .map(|e| match e {
            ExportStructInput::TypeExpr(t, _) => t,
            ExportStructInput::TypeExprFact(t, _, _) => t,
        })
        .collect::<Vec<_>>();
    let module = models::Module::from((
        ident,
        module_pub_path.as_ref(),
        struct_type_exports.as_slice(),
    ));
    repository::ensure(module);
    let Generics { params: generic_params, keys: generic_keys, prov_lifetimes, where_predicates } = Generics::from_input(&input);
    let struct_export_outputs = struct_exports_by_type
        .values()
        .map(|exports| {
            let values = exports
                .iter()
                .map(|e| {
                    let (mut ty, inputs, value) = match e.to_owned().to_owned() {
                        ExportStructInput::TypeExpr(t, v) => (t, vec![], v),
                        ExportStructInput::TypeExprFact(t, i, v) => (t, i, v),
                    };
                    super::core::substitute_in_type(
                        &mut ty,
                        "Self",
                        ident.to_string().as_str(),
                    );
                    (ty, inputs, value)
                })
                .collect::<Vec<_>>();
            let iter_match_outputs =
                values
                    .iter()
                    .enumerate()
                    .map(|(index, (_, inputs, value))| {
                        quote! {
                            #index => {
                                #(let #inputs = provider.provide();)*
                                #value
                            },
                        }
                    });
            let prov_types = values
                .iter()
                .flat_map(|(_, inputs, _)| inputs.iter().map(|i| &i.ty));
            let ty = &values.first().unwrap().0;
            let iter_output = quote! {
                impl<'prov, #(#generic_params,)*NjectProvider> nject::RefIterable<'prov, #ty, NjectProvider> for #ident<#(#generic_keys),*>
                    where
                        #prov_lifetimes
                        NjectProvider: #(nject::Provider<'prov, #prov_types>)+*, #where_predicates
                {
                    #[inline]
                    fn inject(&'prov self, provider: &'prov NjectProvider, index: usize) -> #ty {
                        match index {
                            #( #iter_match_outputs )*
                            _ => unreachable!("Invalid index {index}"),
                        }
                    }
                }
            };

            let (ty, inputs, value) = values.last().unwrap();
            let prov_types = inputs.iter().map(|i| &i.ty);
            quote! {

                impl<'prov, #(#generic_params,)*NjectProvider> nject::RefInjectable<'prov, #ty, NjectProvider> for #ident<#(#generic_keys),*>
                    where
                        #prov_lifetimes
                        NjectProvider: #(nject::Provider<'prov, #prov_types>)+*, #where_predicates
                {
                    #[inline]
                    fn inject(&'prov self, provider: &'prov NjectProvider) -> #ty {
                        #(let #inputs = provider.provide();)*
                        #value
                    }
                }

                #iter_output
            }
        });

    let export_outputs = export_attr_indexes.iter().map(|(i, attrs)| {
        let field = fields[*i];
        let ref_prefix = if let Type::Reference(r) = &field.ty {
            let lifetime = &r.lifetime;
            quote! { &#lifetime }
        } else {
            quote! { &'prov }
        };
        let inputs = attrs.iter().map(|a| match a.meta {
            syn::Meta::Path(_) => ExportFieldInput::Type(field.ty.to_owned()),
            _ => a.parse_args::<ExportFieldInput>().unwrap(),
        });
        let index = syn::Index::from(*i);
        let field_key = match &field.ident {
            Some(i) => quote! { #i },
            None => quote! { #index },
        };
        let outputs = inputs.map(|input| {
            let ty = match &input {
                ExportFieldInput::None => match &field.ty {
                    Type::Reference(r) => {
                        let inner_ty = &r.elem;
                        quote! { #ref_prefix #inner_ty }
                    }
                    _ => {
                        let ty = &field.ty;
                        quote! { #ref_prefix #ty }
                    }
                },
                ExportFieldInput::Type(t) => match t {
                    Type::Reference(r) => {
                        let inner_ty = &r.elem;
                        quote! { #ref_prefix #inner_ty }
                    }
                    _ => quote! { #ref_prefix #t },
                },
                ExportFieldInput::TypeExpr(t, _, _) => quote! { #t },
            };

            let body = match &input {
                ExportFieldInput::TypeExpr(_, i, e) => {
                    let ref_prefix = match &field.ty {
                        Type::Reference(_) => quote! {},
                        _ => quote! { & },
                    };
                    quote! {
                        let #i = #ref_prefix provider.reference(). #field_key;
                        #e
                    }
                }
                _ => quote! { & provider.reference(). #field_key },
            };
            quote! {
                #[allow(non_local_definitions)]
                impl<'prov, #(#generic_params,)*NjectProvider> nject::Injectable<'prov, #ty, NjectProvider> for #ty
                    where
                        #prov_lifetimes
                        NjectProvider: nject::Import<#ident<#(#generic_keys),*>>, #where_predicates
                {
                    #[inline]
                    fn inject(provider: &'prov NjectProvider) -> #ty {
                        #body
                    }
                }
            }
        });
        quote! {
            #(#outputs)*
        }
    });

    let output = quote! {
        #[derive(nject::ModuleHelperAttr)]
        #input
        #(#struct_export_outputs)*
        #(#export_outputs)*
    };
    Ok(output.into())
}
