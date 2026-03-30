mod decorators;
mod imports;
mod provides;
mod scope;

use crate::attrs::{ParsedField, ProviderAttrs};
use crate::core::{DeriveInputExt, Generics};
use quote::quote;

pub(crate) fn handle_provider(
    item: proc_macro::TokenStream,
) -> syn::Result<proc_macro::TokenStream> {
    let input = syn::parse::<syn::DeriveInput>(item)?;
    let ident = &input.ident;
    let fields = input.fields().iter().collect::<Vec<_>>();
    let Generics { params: generic_params, keys: generic_keys, where_predicates, .. } = Generics::from_input(&input);

    let parsed_fields = ParsedField::from_fields(input.fields().iter())
        .map_err(syn::Error::from)?;

    let import_attr_indexes: Vec<usize> = parsed_fields
        .iter()
        .enumerate()
        .filter_map(|(i, pf)| if pf.import { Some(i) } else { None })
        .collect();

    let provide_attr_indexes: Vec<(usize, Vec<&syn::Attribute>)> = parsed_fields
        .iter()
        .enumerate()
        .filter_map(|(i, pf)| {
            if !pf.has_provide_or_singleton() {
                return None;
            }
            let attrs = fields[i]
                .attrs
                .iter()
                .filter(|a| a.path().is_ident("provide") || a.path().is_ident("singleton"))
                .collect::<Vec<_>>();
            Some((i, attrs))
        })
        .collect();

    let provider_attrs = ProviderAttrs::from_attrs(&input.attrs);
    let provide_input_attr: Vec<&syn::Attribute> = provider_attrs.provide_attrs.iter().collect();
    let decorate_input_attr: Vec<&syn::Attribute> = provider_attrs.decorate_attrs.iter().collect();
    let scope_attr: Vec<&syn::Attribute> = provider_attrs.scope_attrs.iter().collect();

    let fields_path_prefix = quote! {};
    let import_outputs = imports::gen_imports_for_import_attr(
        ident,
        &generic_params,
        &generic_keys,
        &where_predicates,
        &fields_path_prefix,
        &fields,
        &import_attr_indexes,
    );
    let provide_outputs = provides::gen_providers_for_provide_attr_on_fields(
        ident,
        &generic_params,
        &generic_keys,
        &where_predicates,
        &fields_path_prefix,
        &fields,
        &provide_attr_indexes,
    );
    let input_provide_outputs = decorators::gen_providers_for_provide_attr_on_struct(
        ident,
        &generic_params,
        &generic_keys,
        &where_predicates,
        &provide_input_attr,
        &decorate_input_attr,
    );
    let scope_output = scope::gen_scope_output(scope::GenScopeOutputInput {
        visibility: &input.vis,
        ident,
        generic_params: &generic_params,
        generic_keys: &generic_keys,
        where_predicates: &where_predicates,
        fields: &fields,
        import_attr_indexes: &import_attr_indexes,
        provide_attr_indexes: &provide_attr_indexes,
        provide_input_attr: &provide_input_attr,
        decorate_input_attr: &decorate_input_attr,
        scope_input_attr: &scope_attr,
    })?;

    let output = quote! {
        #[derive(nject::ProviderHelperAttr)]
        #input

        impl<'prov, #(#generic_params,)*Njecty> nject::Provider<'prov, Njecty> for #ident<#(#generic_keys),*>
        where Njecty: nject::Injectable<'prov, Njecty, #ident<#(#generic_keys),*>>, #where_predicates
        {
            #[inline]
            fn provide(&'prov self) -> Njecty {
                Njecty::inject(self)
            }
        }

        impl<'prov, #(#generic_params,)*Njecty> nject::Provider<'prov, &'prov dyn nject::Provider<'prov, Njecty>> for #ident<#(#generic_keys),*>
        where Self: nject::Provider<'prov, Njecty>, #where_predicates
        {
            #[inline]
            fn provide(&'prov self) -> &'prov dyn nject::Provider<'prov, Njecty> {
                self
            }
        }

        impl<'prov, #(#generic_params,)*Njecty> nject::AsyncProvider<'prov, Njecty> for #ident<#(#generic_keys),*>
        where Njecty: nject::AsyncInjectable<'prov, Njecty, #ident<#(#generic_keys),*>>, #where_predicates
        {
            #[inline]
            fn provide(&'prov self) -> impl core::future::Future<Output = Njecty> {
                Njecty::inject(self)
            }
        }

        impl<#(#generic_params),*> #ident<#(#generic_keys),*>
        where #where_predicates
        {
            #[inline]
            pub fn provide<'prov, Njecty>(&'prov self) -> Njecty
            where Self: nject::Provider<'prov, Njecty>
            {
                <Self as nject::Provider<'prov, Njecty>>::provide(self)
            }

            #[inline]
            pub fn provide_async<'prov, Njecty>(&'prov self) -> impl core::future::Future<Output = Njecty>
            where Self: nject::AsyncProvider<'prov, Njecty>
            {
                <Self as nject::AsyncProvider<'prov, Njecty>>::provide(self)
            }

            #[inline]
            pub fn iter<'prov, Value>(&'prov self) -> impl Iterator<Item = Value> + use<'prov #(,#generic_keys)*, Value>
            where Self: nject::Iterable<'prov, Value>
            {
                nject::Iterable::<'prov, Value>::iter(self)
            }
        }
        #(#import_outputs)*
        #(#provide_outputs)*
        #(#input_provide_outputs)*

        #scope_output
    };
    Ok(output.into())
}
