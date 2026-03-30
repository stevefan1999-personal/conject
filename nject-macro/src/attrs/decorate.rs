use syn::{
    Ident, Token, Type,
    parse::{Parse, ParseStream},
    spanned::Spanned,
};

/// Parsed representation of `#[decorate(Type, |var| expr)]`.
/// The closure takes the already-provided value and wraps/decorates it.
pub struct DecorateStructInput {
    pub ty: Type,
    pub var: Ident,
    pub expr: Box<syn::Expr>,
}

impl Parse for DecorateStructInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let parsed_type: Type = input.parse()?;
        input.parse::<Token![,]>()?;
        let closure: syn::ExprClosure = input.parse()?;
        if closure.inputs.len() != 1 {
            return Err(syn::Error::new(
                closure.span(),
                "Decorate closure must have exactly one parameter.",
            ));
        }
        let param = &closure.inputs[0];
        let var = match param {
            syn::Pat::Ident(pat_ident) => pat_ident.ident.clone(),
            syn::Pat::Type(pat_type) if let syn::Pat::Ident(pat_ident) = &*pat_type.pat => {
                pat_ident.ident.clone()
            }
            _ => {
                return Err(syn::Error::new(
                    param.span(),
                    "Decorate closure parameter must be an identifier.",
                ));
            }
        };
        Ok(DecorateStructInput {
            ty: parsed_type,
            var,
            expr: closure.body,
        })
    }
}
