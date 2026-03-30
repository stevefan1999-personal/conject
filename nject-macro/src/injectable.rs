use crate::core::{DeriveInput, FactoryExpr, error};
use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Expr, PatType, Token, Type,
    parse::{Parse, ParseStream},
    spanned::Spanned,
};

enum InjectExpr {
    /// A direct expression, optionally with factory inputs: `expr` or `|dep: T| expr`
    Value(Box<Expr>, Vec<PatType>),
    /// A named injection tag type: `named(TagType)`
    Named(Type),
    /// A named injection string key: `named("key")` — resolved via `Key<{hash}>`
    NamedStr(u128),
}
impl Parse for InjectExpr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Check for `named(TagType)` or `named("string_key")` syntax
        if input.peek(syn::Ident) {
            let fork = input.fork();
            if let Ok(ident) = fork.parse::<syn::Ident>() {
                if ident == "named" {
                    // Commit to the `named(...)` parse
                    input.parse::<syn::Ident>()?; // consume "named"
                    let content;
                    syn::parenthesized!(content in input);
                    // Check for string literal: named("key")
                    if content.peek(syn::LitStr) {
                        let lit: syn::LitStr = content.parse()?;
                        let hash_bytes = crate::core::hash::fnv(lit.value().as_bytes());
                        let hash = u128::from_be_bytes(hash_bytes);
                        return Ok(InjectExpr::NamedStr(hash));
                    }
                    // Otherwise parse as type: named(TagType)
                    let tag_type: Type = content.parse()?;
                    return Ok(InjectExpr::Named(tag_type));
                }
            }
        }
        if input.peek(Token![|]) {
            let expr = FactoryExpr::parse(input)?;
            Ok(InjectExpr::Value(expr.body, expr.inputs))
        } else {
            Ok(InjectExpr::Value(input.parse()?, vec![]))
        }
    }
}

/// Check if a type's last path segment matches the given name (e.g. "Lazy").
fn is_type_named(ty: &Type, name: &str) -> bool {
    match ty {
        Type::Path(type_path) => type_path
            .path
            .segments
            .last()
            .map(|seg| seg.ident == name)
            .unwrap_or(false),
        _ => false,
    }
}

pub(crate) fn handle_injectable(item: TokenStream) -> syn::Result<TokenStream> {
    let mut input = syn::parse::<DeriveInput>(item)?;
    let post_construct = input
        .0
        .attrs
        .iter()
        .find(|a| a.path().is_ident("post_construct"))
        .map(|a| a.parse_args::<Expr>())
        .transpose()?;
    let pre_destroy = input
        .0
        .attrs
        .iter()
        .find(|a| a.path().is_ident("pre_destroy"))
        .map(|a| a.parse_args::<Expr>())
        .transpose()?;
    input
        .0
        .attrs
        .retain(|a| !a.path().is_ident("post_construct") && !a.path().is_ident("pre_destroy"));
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
            attr.parse_args::<InjectExpr>().map(Some).map_err(|e| {
                error::combine(
                    syn::Error::new(attr.span(), "Unable to parse inject attribute"),
                    e,
                )
            })
        })
        .collect::<syn::Result<Vec<_>>>()?;

    // Detect #[assisted] fields
    let assisted_flags: Vec<bool> = fields
        .iter()
        .map(|f| f.attrs.iter().any(|a| a.path().is_ident("assisted")))
        .collect();
    let has_assisted = assisted_flags.iter().any(|&x| x);

    // Validate: a field cannot be both #[inject] and #[assisted]
    for (i, f) in fields.iter().enumerate() {
        if assisted_flags[i] && attributes[i].is_some() {
            return Err(syn::Error::new(
                f.attrs
                    .iter()
                    .find(|a| a.path().is_ident("assisted"))
                    .unwrap()
                    .span(),
                "A field cannot have both #[inject] and #[assisted] attributes",
            ));
        }
    }

    let generic_params = input.generic_params();
    let generic_keys = input.generic_keys();
    let lifetime_keys = input.lifetime_keys();
    let prov_lifetimes = match lifetime_keys.is_empty() {
        false => quote! { 'prov: #(#lifetime_keys)+*, },
        true => quote! {},
    };
    let where_predicates = match &input.generics.where_clause {
        Some(w) => {
            let predicates = &w.predicates;
            quote! { #predicates }
        }
        None => quote! {},
    };

    if has_assisted {
        return handle_assisted_injectable(
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

    let creation_output = match keys.is_empty() && !types.is_empty() {
        true => {
            let items = types.iter().zip(&attributes).map(|(ty, a)| match a {
                Some(InjectExpr::Named(tag)) => {
                    quote! { nject::Named::<#tag, #ty>::into_inner(provider.provide()) }
                }
                Some(InjectExpr::NamedStr(hash)) => {
                    quote! { nject::Named::<nject::Key<#hash>, #ty>::into_inner(provider.provide()) }
                }
                Some(InjectExpr::Value(output, inputs)) => {
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
                }
                None if is_type_named(ty, "Option") => quote! { None },
                None if is_type_named(ty, "Late") => quote! { nject::Late::new() },
                None if is_type_named(ty, "Lazy") => quote! { nject::Lazy::new() },
                None => quote! { provider.provide() },
            });
            quote! { #ident(#(#items),*) }
        }
        false => {
            let items = keys.iter().zip(types.iter()).zip(&attributes).map(|((k, ty), a)| match a {
                Some(InjectExpr::Named(tag)) => {
                    quote! {
                        #k: nject::Named::<#tag, #ty>::into_inner(provider.provide())
                    }
                }
                Some(InjectExpr::NamedStr(hash)) => {
                    quote! {
                        #k: nject::Named::<nject::Key<#hash>, #ty>::into_inner(provider.provide())
                    }
                }
                Some(InjectExpr::Value(output, inputs)) => {
                    let input_stmts = inputs
                        .iter()
                        .map(|x| quote! { let #x = provider.provide(); })
                        .collect::<Vec<_>>();
                    quote! {
                        #k: {
                            #(#input_stmts)*
                            #output
                        }
                    }
                }
                None if is_type_named(ty, "Option") => quote! { #k: None },
                None if is_type_named(ty, "Late") => quote! { #k: nject::Late::new() },
                None if is_type_named(ty, "Lazy") => quote! { #k: nject::Lazy::new() },
                None => quote! { #k: provider.provide() },
            });
            quote! { #ident { #(#items),* } }
        }
    };
    let creation_output = match &post_construct {
        Some(expr) => quote! { (#expr)(#creation_output) },
        None => creation_output,
    };
    let mut prov_types = Vec::<_>::with_capacity(types.len());
    for (t, a) in types.iter().zip(&attributes) {
        match a {
            Some(InjectExpr::Named(tag)) => {
                prov_types.push(quote! { nject::Named<#tag, #t> });
            }
            Some(InjectExpr::NamedStr(hash)) => {
                prov_types.push(quote! { nject::Named<nject::Key<#hash>, #t> });
            }
            Some(InjectExpr::Value(_, inputs)) => {
                for attr_type in inputs.iter().map(|x| &x.ty) {
                    prov_types.push(quote! {#attr_type});
                }
            }
            None if !is_type_named(t, "Option") && !is_type_named(t, "Late") && !is_type_named(t, "Lazy") => {
                prov_types.push(quote! {#t});
            }
            None => {}
        }
    }
    prov_types.dedup_by(|a, b| a.to_string() == b.to_string());
    let provider_bounds = if prov_types.is_empty() {
        quote! {}
    } else {
        quote! { NjectProvider: #(nject::Provider<'prov, #prov_types>)+*, }
    };
    let pre_destroy_output = match &pre_destroy {
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

        #pre_destroy_output
    };
    Ok(output.into())
}

#[allow(clippy::too_many_arguments)]
fn handle_assisted_injectable(
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
                keys.iter()
                    .nth(i)
                    .copied()
                    .expect("named field should have ident")
                    .clone()
            };
            assisted_params.push(quote! { #param_name: #ty });
        } else if let Some(inject_attr) = attr {
            if let InjectExpr::Value(_, inputs) = inject_attr {
                for attr_type in inputs.iter().map(|x| &x.ty) {
                    prov_types.push(quote! { #attr_type });
                }
            }
        } else {
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
                } else {
                    match attr {
                        Some(InjectExpr::Value(output, inputs)) => {
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
                        }
                        Some(_) => quote! { provider.provide() },
                        None => quote! { provider.provide() },
                    }
                }
            });
        quote! { #ident(#(#items),*) }
    } else {
        // For named struct, track a separate index into the keys slice
        // keys only contains named fields, and for a named struct all fields are named,
        // so keys[i] corresponds to fields[i]
        let items = fields
            .iter()
            .enumerate()
            .map(|(i, _f)| {
                let key = keys[i];
                if assisted_flags[i] {
                    quote! { #key }
                } else {
                    match &attributes[i] {
                        Some(InjectExpr::Value(output, inputs)) => {
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
                        }
                        Some(_) => quote! { #key: provider.provide() },
                        None => quote! { #key: provider.provide() },
                    }
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
