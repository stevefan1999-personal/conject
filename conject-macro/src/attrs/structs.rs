use darling::Error as DarlingError;

/// Parsed struct-level attributes for `#[injectable]`.
pub struct InjectableAttrs {
    pub post_construct: Option<syn::Expr>,
    pub pre_destroy: Option<syn::Expr>,
    pub async_pre_destroy: Option<syn::Expr>,
}

impl InjectableAttrs {
    pub fn from_attrs(attrs: &[syn::Attribute]) -> Result<Self, darling::Error> {
        let (mut post_construct, mut pre_destroy, mut async_pre_destroy) = (None, None, None);
        let mut errors = DarlingError::accumulator();
        for attr in attrs {
            let (target, name) = if attr.path().is_ident("post_construct") {
                (&mut post_construct, "post_construct")
            } else if attr.path().is_ident("pre_destroy") {
                (&mut pre_destroy, "pre_destroy")
            } else if attr.path().is_ident("async_pre_destroy") {
                (&mut async_pre_destroy, "async_pre_destroy")
            } else {
                continue;
            };
            match attr.parse_args::<syn::Expr>() {
                Ok(expr) => *target = Some(expr),
                Err(e) => errors.push(
                    DarlingError::custom(format!("Unable to parse {name} attribute: {e}"))
                        .with_span(attr),
                ),
            }
        }
        errors.finish()?;
        Ok(Self {
            post_construct,
            pre_destroy,
            async_pre_destroy,
        })
    }

    pub fn strip_from(attrs: &mut Vec<syn::Attribute>) {
        attrs.retain(|a| {
            !a.path().is_ident("post_construct")
                && !a.path().is_ident("pre_destroy")
                && !a.path().is_ident("async_pre_destroy")
        });
    }
}

/// Parsed struct-level attributes for `#[provider]`.
pub struct ProviderAttrs {
    pub provide_attrs: Vec<syn::Attribute>,
    pub decorate_attrs: Vec<syn::Attribute>,
    pub scope_attrs: Vec<syn::Attribute>,
}

impl ProviderAttrs {
    pub fn from_attrs(attrs: &[syn::Attribute]) -> Self {
        let (mut provide_attrs, mut decorate_attrs, mut scope_attrs) = (vec![], vec![], vec![]);
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
