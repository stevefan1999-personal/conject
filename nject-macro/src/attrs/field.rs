use super::inject::InjectExpr;
use crate::injectable::creation::is_type_named;
use darling::Error as DarlingError;

/// Centralized parsed field info combining attribute detection with manual expression parsing.
///
/// darling's `FromField` cannot handle nject's raw expression syntax (`#[inject(MyStruct { value: 42 })]`),
/// so we use darling's `Error` type for accumulating errors while parsing manually.
///
/// All fields are public for downstream consumption; some may not be read by all callers.
#[derive(Clone)]
#[allow(dead_code)]
pub struct ParsedField {
    pub ident: Option<syn::Ident>,
    pub ty: syn::Type,
    pub vis: syn::Visibility,
    /// Parsed `#[inject(...)]` expression, if present
    pub inject: Option<InjectExpr>,
    /// Whether this field has `#[import]`
    pub import: bool,
    /// Whether this field has `#[assisted]`
    pub assisted: bool,
    /// Whether this field has `#[singleton]`
    pub singleton: bool,
    /// Raw `#[provide]` / `#[provide(...)]` attributes on this field
    pub provide_attrs: Vec<syn::Attribute>,
    /// Raw `#[export]` / `#[export(...)]` attributes on this field
    pub export_attrs: Vec<syn::Attribute>,
}

impl ParsedField {
    /// Parse a `syn::Field` into a `ParsedField`, extracting all nject-relevant attributes.
    ///
    /// Uses `darling::Error` for error accumulation so multiple errors can be reported at once.
    pub fn from_field(field: &syn::Field) -> Result<Self, darling::Error> {
        let mut inject = None;
        let mut import = false;
        let mut assisted = false;
        let mut singleton = false;
        let mut provide_attrs = Vec::new();
        let mut export_attrs = Vec::new();
        let mut errors = DarlingError::accumulator();

        for attr in &field.attrs {
            let path = attr.path();
            if path.is_ident("inject") {
                match attr.parse_args::<InjectExpr>() {
                    Ok(expr) => inject = Some(expr),
                    Err(e) => errors.push(
                        DarlingError::custom(format!("Unable to parse inject attribute: {e}"))
                            .with_span(attr),
                    ),
                }
            } else if path.is_ident("import") {
                import = true;
            } else if path.is_ident("assisted") {
                assisted = true;
            } else if path.is_ident("singleton") {
                singleton = true;
            } else if path.is_ident("provide") {
                provide_attrs.push(attr.clone());
            } else if path.is_ident("export") {
                export_attrs.push(attr.clone());
            }
        }

        // Validate: a field cannot have both #[inject] and #[assisted]
        if assisted && inject.is_some() {
            errors.push(
                DarlingError::custom("A field cannot have both #[inject] and #[assisted] attributes")
                    .with_span(
                        field
                            .attrs
                            .iter()
                            .find(|a| a.path().is_ident("assisted"))
                            .unwrap_or(&field.attrs[0]),
                    ),
            );
        }

        errors.finish()?;

        Ok(Self {
            ident: field.ident.clone(),
            ty: field.ty.clone(),
            vis: field.vis.clone(),
            inject,
            import,
            assisted,
            singleton,
            provide_attrs,
            export_attrs,
        })
    }

    /// Parse all fields from a `syn::Fields` iterator.
    ///
    /// Accumulates errors across all fields using `darling::Error::multiple`.
    pub fn from_fields<'a>(
        fields: impl Iterator<Item = &'a syn::Field>,
    ) -> Result<Vec<Self>, darling::Error> {
        let mut parsed = Vec::new();
        let mut errors = Vec::new();

        for field in fields {
            match Self::from_field(field) {
                Ok(pf) => parsed.push(pf),
                Err(e) => errors.push(e),
            }
        }

        if errors.is_empty() {
            Ok(parsed)
        } else {
            Err(DarlingError::multiple(errors))
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
    pub fn has_export(&self) -> bool {
        !self.export_attrs.is_empty()
    }
}
