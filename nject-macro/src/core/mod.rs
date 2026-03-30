use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use std::path::PathBuf;
use std::str::FromStr;
use syn::{
    Expr, ExprClosure, Fields, GenericArgument, GenericParam,
    Ident, Pat, PatType, Path, PathSegment, Token, Type,
    parse::{Parse, ParseStream},
    spanned::Spanned,
};

pub(crate) trait DeriveInputExt {
    fn fields(&self) -> &Fields;
    fn field_types(&self) -> Vec<&Type>;
    fn field_idents(&self) -> Vec<&Ident>;
    fn generic_params(&self) -> Vec<&GenericParam>;
    fn generic_keys(&self) -> Vec<TokenStream>;
    fn lifetime_keys(&self) -> Vec<TokenStream>;
}

impl DeriveInputExt for syn::DeriveInput {
    fn fields(&self) -> &Fields {
        match &self.data {
            syn::Data::Struct(d) => &d.fields,
            _ => panic!("Unsupported type. Macro should be used on a struct"),
        }
    }
    fn field_types(&self) -> Vec<&Type> {
        self.fields().iter().map(|f| &f.ty).collect::<Vec<_>>()
    }
    fn field_idents(&self) -> Vec<&Ident> {
        self.fields()
            .iter()
            .filter_map(|f| f.ident.as_ref())
            .collect::<Vec<_>>()
    }
    fn generic_params(&self) -> Vec<&GenericParam> {
        self.generics.params.iter().collect::<Vec<_>>()
    }
    fn generic_keys(&self) -> Vec<TokenStream> {
        self.generics
            .params
            .iter()
            .map(|p| match p {
                GenericParam::Type(t) => {
                    let identity = &t.ident;
                    quote! { #identity }
                }
                GenericParam::Const(c) => {
                    let identity = &c.ident;
                    quote! { #identity }
                }
                GenericParam::Lifetime(l) => quote! { #l },
            })
            .collect::<Vec<_>>()
    }
    fn lifetime_keys(&self) -> Vec<TokenStream> {
        self.generics
            .params
            .iter()
            .filter_map(|p| {
                if let GenericParam::Lifetime(l) = p {
                    Some(quote! { #l })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
    }
}

pub(crate) struct Generics<'a> {
    pub params: Vec<&'a GenericParam>,
    pub keys: Vec<TokenStream>,
    pub prov_lifetimes: TokenStream,
    pub where_predicates: TokenStream,
}

impl<'a> Generics<'a> {
    pub fn from_input(input: &'a syn::DeriveInput) -> Self {
        let params = input.generic_params();
        let keys = input.generic_keys();
        let lifetime_keys = input.lifetime_keys();
        let prov_lifetimes = if lifetime_keys.is_empty() {
            quote! {}
        } else {
            quote! { 'prov: #(#lifetime_keys)+*, }
        };
        let where_predicates = match &input.generics.where_clause {
            Some(w) => {
                let predicates = &w.predicates;
                quote! { #predicates }
            }
            None => quote! {},
        };
        Self { params, keys, prov_lifetimes, where_predicates }
    }
}

pub struct FactoryExpr {
    pub inputs: Vec<PatType>,
    pub body: Box<Expr>,
}

impl Parse for FactoryExpr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let expr: ExprClosure = input.parse()?;
        let mut inputs = Vec::with_capacity(expr.inputs.len());
        let span = expr.span();
        for input in expr.inputs {
            if let Pat::Type(pat_type) = input {
                inputs.push(pat_type);
            } else {
                return Err(syn::Error::new(
                    span,
                    format!("Invalid input: {}", input.to_token_stream()),
                ));
            }
        }
        Ok(FactoryExpr {
            inputs,
            body: expr.body,
        })
    }
}

pub enum FieldFactoryExpr {
    None,
    Type(Type),
    TypeExpr(Type, Ident, Box<Expr>),
}
impl Parse for FieldFactoryExpr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.is_empty() {
            return Ok(Self::None);
        }
        let parsed_type = input.parse()?;
        if !input.peek(Token![,]) {
            return Ok(Self::Type(parsed_type));
        }

        input.parse::<Token![,]>()?;
        let expr = input.parse::<ExprClosure>()?;
        if expr.inputs.is_empty() {
            return Err(syn::Error::new(expr.span(), "Missing factory input."));
        }
        if expr.inputs.len() > 1 {
            return Err(syn::Error::new(expr.span(), "More than one input found"));
        }

        let input = &expr.inputs[0];
        let Pat::Ident(pat_ident) = input else {
            return Err(syn::Error::new(input.span(), "Input must be an identity."));
        };
        Ok(Self::TypeExpr(
            parsed_type,
            pat_ident.ident.to_owned(),
            expr.body,
        ))
    }
}

pub fn extract_path_from_type(ty: &Type) -> &Path {
    match ty {
        Type::Path(p) => &p.path,
        Type::Reference(r) => extract_path_from_type(&r.elem),
        _ => panic!("Unsupported type. Must be a Path or a Reference type."),
    }
}

pub fn cache_path() -> PathBuf {
    let out_dir = env!("NJECT_OUT_DIR");
    std::path::PathBuf::from_str(out_dir).expect("Unable to construct NJECT_OUT_DIR")
}

pub fn retry<T, E>(times: usize, action: impl Fn() -> Result<T, E>) -> Result<T, E> {
    let result = action();
    if result.is_ok() || times < 1 { result }
    else {
        std::thread::sleep(std::time::Duration::from_millis(100));
        retry(times - 1, action)
    }
}

pub fn substitute_in_path(path: &mut Path, from: &str, to: &str) {
    for segment in path.segments.iter_mut() {
        substitute_in_path_segment(segment, from, to)
    }
}

pub fn substitute_in_type(ty: &mut Type, from: &str, to: &str) {
    match ty {
        Type::Path(p) => substitute_in_path(&mut p.path, from, to),
        Type::Reference(r) => substitute_in_type(&mut r.elem, from, to),
        Type::TraitObject(t) => {
            for bound in &mut t.bounds {
                if let syn::TypeParamBound::Trait(t) = bound {
                    substitute_in_path(&mut t.path, from, to)
                }
            }
        }
        _ => panic!("Unsupported type: {}", ty.to_token_stream()),
    };
}

fn substitute_in_path_segment(segment: &mut PathSegment, from: &str, to: &str) {
    if segment.ident == from {
        segment.ident = syn::Ident::new(to, segment.ident.span());
    }
    match &mut segment.arguments {
        syn::PathArguments::None => (),
        syn::PathArguments::AngleBracketed(b) => {
            for arg in &mut b.args { substitute_in_generic_argument(arg, from, to) }
        }
        syn::PathArguments::Parenthesized(p) => {
            for ty in &mut p.inputs { substitute_in_type(ty, from, to) }
        }
    };
}

fn substitute_in_generic_argument(arg: &mut GenericArgument, from: &str, to: &str) {
    match arg {
        syn::GenericArgument::Type(ty) => substitute_in_type(ty, from, to),
        syn::GenericArgument::AssocType(a) => {
            if let Some(args) = &mut a.generics {
                for arg in &mut args.args { substitute_in_generic_argument(arg, from, to) }
            }
            substitute_in_type(&mut a.ty, from, to)
        }
        syn::GenericArgument::Constraint(c) => {
            if let Some(args) = &mut c.generics {
                for arg in &mut args.args { substitute_in_generic_argument(arg, from, to) }
            }
            for bound in &mut c.bounds {
                if let syn::TypeParamBound::Trait(t) = bound {
                    substitute_in_path(&mut t.path, from, to)
                }
            }
        }
        _ => (),
    }
}
