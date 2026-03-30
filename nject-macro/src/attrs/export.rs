use crate::core::{FactoryExpr, FieldFactoryExpr};
use syn::{
    Expr, PatType, Token, Type,
    parse::{Parse, ParseStream},
};

/// Parsed representation of struct-level `#[export(Type, expr)]` or `#[export(Type, |dep: T| expr)]`.
#[derive(Clone)]
pub enum ExportStructInput {
    TypeExpr(Type, Box<Expr>),
    TypeExprFact(Type, Vec<PatType>, Box<Expr>),
}

impl Parse for ExportStructInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let parsed_type = input.parse()?;
        input.parse::<Token![,]>()?;
        if input.peek(Token![|]) {
            let expr = FactoryExpr::parse(input)?;
            Ok(Self::TypeExprFact(parsed_type, expr.inputs, expr.body))
        } else {
            Ok(Self::TypeExpr(parsed_type, input.parse()?))
        }
    }
}

/// Field-level `#[export]` / `#[export(Type)]` / `#[export(Type, |var| expr)]`.
pub type ExportFieldInput = FieldFactoryExpr;
