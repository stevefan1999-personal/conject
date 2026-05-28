use super::inject::InjectExpr;
use darling::FromField;

#[derive(Clone, FromField)]
#[darling(forward_attrs)]
pub struct ParsedField {
    pub attrs: Vec<syn::Attribute>,
    #[darling(skip)]
    pub inject: Option<InjectExpr>,
    #[darling(skip)]
    pub import: bool,
    #[darling(skip)]
    pub assisted: bool,
    #[darling(skip)]
    pub singleton: bool,
    #[darling(skip)]
    pub late_bind: Option<syn::Expr>,
    #[darling(skip)]
    pub provide_attrs: Vec<syn::Attribute>,
    #[darling(skip)]
    pub export_attrs: Vec<syn::Attribute>,
}

impl ParsedField {
    pub fn from_syn_field(field: &syn::Field) -> darling::Result<Self> {
        let mut parsed = Self::from_field(field)?;
        let mut errors = darling::Error::accumulator();
        for attr in &parsed.attrs {
            let path = attr.path();
            if path.is_ident("inject") {
                match attr.parse_args::<InjectExpr>() {
                    Ok(expr) => parsed.inject = Some(expr),
                    Err(e) => errors.push(
                        darling::Error::custom(format!("Unable to parse inject attribute: {e}"))
                            .with_span(attr),
                    ),
                }
            } else if path.is_ident("import") {
                parsed.import = true;
            } else if path.is_ident("assisted") {
                parsed.assisted = true;
            } else if path.is_ident("provide") || path.is_ident("singleton") {
                parsed.provide_attrs.push(attr.clone());
                if path.is_ident("singleton") {
                    parsed.singleton = true;
                }
            } else if path.is_ident("export") {
                parsed.export_attrs.push(attr.clone());
            } else if path.is_ident("late_bind") {
                match attr.parse_args::<syn::Expr>() {
                    Ok(expr) => parsed.late_bind = Some(expr),
                    Err(e) => errors.push(
                        darling::Error::custom(format!("Unable to parse late_bind attribute: {e}"))
                            .with_span(attr),
                    ),
                }
            }
        }
        if parsed.assisted && parsed.inject.is_some() {
            errors.push(
                darling::Error::custom(
                    "A field cannot have both #[inject] and #[assisted] attributes",
                )
                .with_span(field),
            );
        }
        errors.finish()?;
        Ok(parsed)
    }

    pub fn from_fields<'a>(
        fields: impl ::core::iter::Iterator<Item = &'a syn::Field>,
    ) -> darling::Result<Vec<Self>> {
        let mut parsed = Vec::new();
        let mut errors = Vec::new();
        for field in fields {
            match Self::from_syn_field(field) {
                Ok(pf) => parsed.push(pf),
                Err(e) => errors.push(e),
            }
        }
        if errors.is_empty() {
            Ok(parsed)
        } else {
            Err(darling::Error::multiple(errors))
        }
    }

    pub fn has_provide_or_singleton(&self) -> bool {
        !self.provide_attrs.is_empty() || self.singleton
    }
}
