use darling::Error as DarlingError;

/// Parsed struct-level attributes for `#[injectable]`.
///
/// Extracts `#[post_construct(expr)]` and `#[pre_destroy(expr)]` from the attribute list.
pub struct InjectableAttrs {
    pub post_construct: Option<syn::Expr>,
    pub pre_destroy: Option<syn::Expr>,
}

impl InjectableAttrs {
    /// Parse injectable-specific attributes from a list of attributes.
    ///
    /// Uses `darling::Error` for accumulating multiple errors.
    pub fn from_attrs(attrs: &[syn::Attribute]) -> Result<Self, darling::Error> {
        let mut post_construct = None;
        let mut pre_destroy = None;
        let mut errors = DarlingError::accumulator();

        for attr in attrs {
            if attr.path().is_ident("post_construct") {
                match attr.parse_args::<syn::Expr>() {
                    Ok(expr) => post_construct = Some(expr),
                    Err(e) => errors.push(
                        DarlingError::custom(format!("Unable to parse post_construct attribute: {e}"))
                            .with_span(attr),
                    ),
                }
            } else if attr.path().is_ident("pre_destroy") {
                match attr.parse_args::<syn::Expr>() {
                    Ok(expr) => pre_destroy = Some(expr),
                    Err(e) => errors.push(
                        DarlingError::custom(format!("Unable to parse pre_destroy attribute: {e}"))
                            .with_span(attr),
                    ),
                }
            }
        }

        errors.finish()?;

        Ok(Self {
            post_construct,
            pre_destroy,
        })
    }

    /// Remove consumed attributes (post_construct, pre_destroy) from a mutable attrs list.
    pub fn strip_from(attrs: &mut Vec<syn::Attribute>) {
        attrs.retain(|a| {
            !a.path().is_ident("post_construct") && !a.path().is_ident("pre_destroy")
        });
    }
}

/// Parsed struct-level attributes for `#[provider]`.
///
/// Collects `#[provide(...)]`, `#[decorate(...)]`, and `#[scope(...)]` attributes.
pub struct ProviderAttrs {
    pub provide_attrs: Vec<syn::Attribute>,
    pub decorate_attrs: Vec<syn::Attribute>,
    pub scope_attrs: Vec<syn::Attribute>,
}

impl ProviderAttrs {
    /// Collect provider-relevant attributes from the struct-level attribute list.
    pub fn from_attrs(attrs: &[syn::Attribute]) -> Self {
        let mut provide_attrs = Vec::new();
        let mut decorate_attrs = Vec::new();
        let mut scope_attrs = Vec::new();

        for attr in attrs {
            let path = attr.path();
            if path.is_ident("provide") {
                provide_attrs.push(attr.clone());
            } else if path.is_ident("decorate") {
                decorate_attrs.push(attr.clone());
            } else if path.is_ident("scope") {
                scope_attrs.push(attr.clone());
            }
        }

        Self {
            provide_attrs,
            decorate_attrs,
            scope_attrs,
        }
    }
}
