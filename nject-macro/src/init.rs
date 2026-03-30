use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    GenericArgument, Ident, Lifetime, Token, Type,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

enum InitInput {
    Expr { modules: Vec<Type> },
    Block { declarations: Vec<LetDecl> },
}

struct LetDecl {
    is_mut: bool,
    ident: Ident,
    ty: Option<Type>,
    modules: Vec<Type>,
}

impl Parse for InitInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(Token![let]) {
            let mut declarations = Vec::new();
            while !input.is_empty() {
                declarations.push(input.parse::<LetDecl>()?);
                // Consume optional semicolon between/after declarations
                while !input.is_empty() && input.peek(Token![;]) {
                    input.parse::<Token![;]>()?;
                }
            }
            Ok(InitInput::Block { declarations })
        } else {
            let modules = Punctuated::<Type, Token![,]>::parse_terminated(input)?;
            Ok(InitInput::Expr {
                modules: modules.into_iter().collect(),
            })
        }
    }
}

impl Parse for LetDecl {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<Token![let]>()?;
        let is_mut = input.peek(Token![mut]);
        if is_mut {
            input.parse::<Token![mut]>()?;
        }
        let ident = input.parse::<Ident>()?;
        let ty = if input.peek(Token![:]) {
            input.parse::<Token![:]>()?;
            Some(input.parse::<Type>()?)
        } else {
            None
        };
        input.parse::<Token![=]>()?;
        let modules = Punctuated::<Type, Token![,]>::parse_separated_nonempty(input)?;
        Ok(LetDecl {
            is_mut,
            ident,
            ty,
            modules: modules.into_iter().collect(),
        })
    }
}

fn collect_lifetimes_from_type(ty: &Type, lifetimes: &mut Vec<Lifetime>) {
    match ty {
        Type::Path(p) => {
            for segment in &p.path.segments {
                let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
                    continue;
                };
                for arg in &args.args {
                    match arg {
                        GenericArgument::Lifetime(lt)
                            if !lifetimes.iter().any(|l| l.ident == lt.ident) =>
                        {
                            lifetimes.push(lt.clone());
                        }
                        GenericArgument::Type(t) => {
                            collect_lifetimes_from_type(t, lifetimes);
                        }
                        _ => {}
                    }
                }
            }
        }
        Type::Reference(r) => {
            if let Some(lt) = &r.lifetime
                && !lifetimes.iter().any(|l| l.ident == lt.ident)
            {
                lifetimes.push(lt.clone());
            }
            collect_lifetimes_from_type(&r.elem, lifetimes);
        }
        _ => {}
    }
}

fn gen_chain(
    modules: &[Type],
    name_prefix: &str,
) -> (
    Vec<proc_macro2::TokenStream>,
    Vec<proc_macro2::TokenStream>,
    Ident,
) {
    let init_ident = format_ident!("__NjectInit_{}", name_prefix);
    let init_var = format_ident!("__nject_init_{}", name_prefix);

    let mut struct_defs = vec![quote! {
        #[nject::provider]
        #[allow(non_camel_case_types)]
        struct #init_ident;
    }];

    let mut let_bindings = vec![quote! {
        let #init_var = #init_ident;
    }];

    let mut last_var = init_var.clone();

    for i in 0..modules.len() - 1 {
        let step_ident = format_ident!("__NjectInitStep_{}_{}", name_prefix, i);
        let step_modules = &modules[0..=i];

        // Collect unique lifetimes from all module types in this step
        let mut lifetimes = Vec::new();
        for m in step_modules {
            collect_lifetimes_from_type(m, &mut lifetimes);
        }

        let generic_params = if lifetimes.is_empty() {
            quote! {}
        } else {
            quote! { <#(#lifetimes),*> }
        };

        let import_fields = step_modules.iter().map(|m| {
            quote! { #[import] #m }
        });

        struct_defs.push(quote! {
            #[nject::injectable]
            #[nject::provider]
            #[allow(non_camel_case_types)]
            struct #step_ident #generic_params (#(#import_fields),*);
        });

        let var_ident = format_ident!("__nject_{}_s{}", name_prefix, i);

        let_bindings.push(quote! {
            let #var_ident = #last_var.provide::<#step_ident>();
        });

        last_var = var_ident;
    }

    (struct_defs, let_bindings, last_var)
}

pub(crate) fn handle_init(item: TokenStream) -> syn::Result<TokenStream> {
    let input = syn::parse::<InitInput>(item)?;

    match input {
        InitInput::Expr { modules } => {
            if modules.is_empty() {
                return Err(syn::Error::new(
                    proc_macro2::Span::call_site(),
                    "init! requires at least one module type",
                ));
            }
            let (struct_defs, let_bindings, last_var) = gen_chain(&modules, "expr");
            Ok(quote! { { #(#struct_defs)* #(#let_bindings)* #last_var.provide() } }.into())
        }
        InitInput::Block { declarations } => {
            if declarations.is_empty() {
                return Err(syn::Error::new(
                    proc_macro2::Span::call_site(),
                    "init! block requires at least one let declaration",
                ));
            }
            let mut all_tokens = Vec::new();
            for decl in &declarations {
                if decl.modules.is_empty() {
                    return Err(syn::Error::new(
                        proc_macro2::Span::call_site(),
                        "each let declaration requires at least one module type",
                    ));
                }
                let mutability = if decl.is_mut {
                    quote! { mut }
                } else {
                    quote! {}
                };
                let ty_annotation = decl
                    .ty
                    .as_ref()
                    .map_or_else(|| quote! {}, |t| quote! { : #t });
                let ident = &decl.ident;
                let (struct_defs, let_bindings, last_var) =
                    gen_chain(&decl.modules, &ident.to_string());
                all_tokens.push(quote! {
                    #(#struct_defs)* #(#let_bindings)*
                    let #mutability #ident #ty_annotation = #last_var.provide();
                });
            }
            Ok(quote! { #(#all_tokens)* }.into())
        }
    }
}
