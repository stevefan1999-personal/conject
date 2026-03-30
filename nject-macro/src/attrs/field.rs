use super::inject::InjectExpr;
use crate::injectable::creation::is_type_named;
use darling::FromField;

/// Centralized parsed field info.
///
/// Uses darling's `FromField` for auto-population of `ident`, `ty`, `vis`
/// (eliminating manual extraction). nject's field attributes (`#[inject]`,
/// `#[import]`, `#[assisted]`, etc.) use a flat `#[attr_name]` style rather
/// than darling's `#[parent(key)]` style, so they are parsed manually.
#[derive(Clone, FromField)]
#[darling(forward_attrs)]
pub struct ParsedField {
    /// Auto-populated by darling from `syn::Field`
    pub ident: Option<syn::Ident>,
    /// Auto-populated by darling from `syn::Field`
    pub ty: syn::Type,
    /// Auto-populated by darling from `syn::Field`
    pub vis: syn::Visibility,
    /// Forwarded attributes — used for manual nject attribute parsing
    pub attrs: Vec<syn::Attribute>,

    // ── Manually parsed nject attributes ──

    /// Parsed `#[inject(...)]` expression
    #[darling(skip)]
    pub inject: Option<InjectExpr>,
    /// `#[import]` flag
    #[darling(skip)]
    pub import: bool,
    /// `#[assisted]` flag
    #[darling(skip)]
    pub assisted: bool,
    /// `#[singleton]` flag (bare or with content)
    #[darling(skip)]
    pub singleton: bool,
    /// Raw `#[provide]` / `#[singleton]` attributes for provider logic
    #[darling(skip)]
    pub provide_attrs: Vec<syn::Attribute>,
    /// Raw `#[export]` attributes for module logic
    #[darling(skip)]
    pub export_attrs: Vec<syn::Attribute>,
}

impl ParsedField {
    /// Parse a field: darling auto-populates ident/ty/vis/attrs,
    /// then we extract nject attributes manually.
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

    /// Parse all fields, accumulating errors across fields.
    pub fn from_fields<'a>(
        fields: impl Iterator<Item = &'a syn::Field>,
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

    /// Check if the field type's last path segment matches the given name.
    #[allow(dead_code)]
    pub fn is_type_named(&self, name: &str) -> bool {
        is_type_named(&self.ty, name)
    }

    /// Whether this field has any `#[provide]` or `#[singleton]` attributes.
    pub fn has_provide_or_singleton(&self) -> bool {
        !self.provide_attrs.is_empty() || self.singleton
    }

    /// Whether this field has any `#[export]` attributes.
    #[allow(dead_code)]
    pub fn has_export(&self) -> bool {
        !self.export_attrs.is_empty()
    }
}
