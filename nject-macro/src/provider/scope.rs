use crate::core::error;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{
    Field, GenericParam, Ident, Lifetime, LifetimeParam, Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
};

pub(super) struct GenScopeOutputInput<'a> {
    pub visibility: &'a syn::Visibility,
    pub ident: &'a Ident,
    pub generic_params: &'a [&'a GenericParam],
    pub generic_keys: &'a [proc_macro2::TokenStream],
    pub where_predicates: &'a proc_macro2::TokenStream,
    pub fields: &'a [&'a syn::Field],
    pub import_attr_indexes: &'a [usize],
    pub provide_attr_indexes: &'a [(usize, Vec<&'a syn::Attribute>)],
    pub provide_input_attr: &'a [&'a syn::Attribute],
    pub decorate_input_attr: &'a [&'a syn::Attribute],
    pub scope_input_attr: &'a [&'a syn::Attribute],
}

pub(super) fn gen_scope_output(
    GenScopeOutputInput {
        visibility,
        ident,
        generic_params,
        generic_keys,
        where_predicates,
        fields,
        import_attr_indexes,
        provide_attr_indexes,
        provide_input_attr,
        decorate_input_attr,
        scope_input_attr,
    }: GenScopeOutputInput<'_>,
) -> syn::Result<proc_macro2::TokenStream> {
    if scope_input_attr.is_empty() {
        return Ok(proc_macro2::TokenStream::new());
    }
    let scope_lifetime = &GenericParam::Lifetime(LifetimeParam {
        lifetime: Lifetime::new("'scope", Span::call_site()),
        attrs: vec![],
        colon_token: None,
        bounds: Punctuated::default(),
    });
    let mut scope_generic_params = vec![scope_lifetime];
    scope_generic_params.extend_from_slice(generic_params);
    let mut scope_generic_keys = vec![quote! {'scope}];
    scope_generic_keys.extend_from_slice(generic_keys);

    let scope_fields = scope_input_attr
        .iter()
        .map(|a| {
            a.parse_args_with(parse_scope_field).map_err(|e| {
                error::combine(syn::Error::new(a.span(), "Unable to parse scope field."), e)
            })
        })
        .collect::<syn::Result<Vec<_>>>()?;
    let grouped_fields = crate::core::collection::group_by(scope_fields.iter(), |k| {
        k.ident.as_ref().map(|i| i.to_string())
    });
    let scope_outputs = grouped_fields.iter().map(|(scope_name, scope_fields)| {
        let scope_ident = match scope_name {
            Some(n) => format_ident!("{}{}Scope", ident, snake_to_pascal(n)),
            None => format_ident!("{ident}Scope"),
        };
        let scope_fn_ident = match scope_name {
            Some(n) => format_ident!("{n}_scope"),
            None => format_ident!("scope"),
        };
        let arg_scope_fields = scope_fields
            .iter()
            .map(|f| {
                f.attrs
                    .iter()
                    .rfind(|a| matches!(&a.meta, syn::Meta::Path(p) if p.is_ident("arg")))
                    .is_some()
            })
            .collect::<Vec<_>>();
        let scope_field_outputs = scope_fields.iter().map(|f| {
            let mut f = f.to_owned().to_owned();
            f.ident = None;
            quote! { #[provide] #f }
        });
        let root_path = syn::Index::from(scope_fields.len());
        let fields_path_prefix = quote! { #root_path. };
        let import_outputs = super::imports::gen_imports_for_import_attr(
            &scope_ident,
            &scope_generic_params,
            &scope_generic_keys,
            where_predicates,
            &fields_path_prefix,
            fields,
            import_attr_indexes,
        );
        let provide_outputs = super::provides::gen_providers_for_provide_attr_on_fields(
            &scope_ident,
            &scope_generic_params,
            &scope_generic_keys,
            where_predicates,
            &fields_path_prefix,
            fields,
            provide_attr_indexes,
        );
        let input_provide_outputs = super::decorators::gen_providers_for_provide_attr_on_struct(
            &scope_ident,
            &scope_generic_params,
            &scope_generic_keys,
            where_predicates,
            provide_input_attr,
            decorate_input_attr,
        );
        let scope_field_provides = scope_fields.iter().enumerate().map(|(i, _)| {
            if arg_scope_fields[i] {
                let ident = format_ident!("v{i}");
                quote! { #ident }
            } else {
                quote! { self.provide() }
            }
        });
        let scope_args = scope_fields
            .iter()
            .enumerate()
            .filter_map(|(i, f)| {
                if arg_scope_fields[i] {
                    let ident = format_ident!("v{i}");
                    let ty = &f.ty;
                    Some(quote! { #ident: #ty })
                } else {
                    None
                }
            });

        quote! {

            impl<#(#generic_params),*> #ident<#(#generic_keys),*>
                where #where_predicates
            {
                #[inline]
                pub fn #scope_fn_ident<'scope>(&'scope self, #(#scope_args),*) -> #scope_ident<#(#scope_generic_keys),*>
                {
                    #scope_ident(#(#scope_field_provides,)* self)
                }
            }

            #[provider]
            #[injectable]
            #[derive(nject::ScopeHelperAttr)]
            #visibility struct #scope_ident<'scope, #(#generic_params),*>(#(#scope_field_outputs,)* &'scope #ident<#(#generic_keys),*>)
                where #where_predicates;

            #(#import_outputs)*
            #(#provide_outputs)*
            #(#input_provide_outputs)*
        }
    });
    Ok(quote! { #(#scope_outputs)* })
}

/// Converts a snake_case string to PascalCase.
fn snake_to_pascal(snake: &str) -> String {
    snake
        .split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

fn parse_scope_field(input: ParseStream) -> syn::Result<Field> {
    if input.peek(Ident) && input.peek2(Token![:]) {
        let ident = Ident::parse(input)?;
        let _token: Token![:] = input.parse()?;
        let mut field = Field::parse_unnamed(input)?;
        field.ident = Some(ident);
        Ok(field)
    } else {
        Field::parse_unnamed(input)
    }
}
