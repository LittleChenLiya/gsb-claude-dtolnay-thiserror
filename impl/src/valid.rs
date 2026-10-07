use crate::ast::{Enum, Field, Input, Struct, Variant};
use crate::attr::Attrs;
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{Error, GenericArgument, PathArguments, Result, Type};

impl Input<'_> {
    pub(crate) fn validate(&self) -> Result<()> {
        match self {
            Input::Struct(input) => input.validate(),
            Input::Enum(input) => input.validate(),
        }
    }
}

impl Struct<'_> {
    fn validate(&self) -> Result<()> {
        check_non_field_attrs(&self.attrs)?;
        if let Some(transparent) = self.attrs.transparent {
            if self.fields.len() != 1 {
                return Err(Error::new_spanned(
                    transparent.original,
                    "#[error(transparent)] requires exactly one field",
                ));
            }
            if let Some(source) = self.fields.iter().find_map(|f| f.attrs.source) {
                return Err(Error::new_spanned(
                    source.original,
                    "transparent error struct can't contain #[source]",
                ));
            }
        }
        if let Some(fmt) = &self.attrs.fmt {
            return Err(Error::new_spanned(
                fmt.original,
                "#[error(fmt = ...)] is only supported in enums; for a struct, handwrite your own Display impl",
            ));
        }
        check_field_attrs(&self.fields)?;
        for field in &self.fields {
            field.validate()?;
        }
        Ok(())
    }
}

impl Enum<'_> {
    fn validate(&self) -> Result<()> {
        check_non_field_attrs(&self.attrs)?;
        let has_display = self.has_display();
        for variant in &self.variants {
            variant.validate()?;
            if has_display
                && variant.attrs.display.is_none()
                && variant.attrs.transparent.is_none()
                && variant.attrs.fmt.is_none()
            {
                return Err(Error::new_spanned(
                    variant.original,
                    "missing #[error(\"...\")] display attribute",
                ));
            }
        }
        check_duplicate_from_types(&self.variants)?;
        Ok(())
    }
}

impl Variant<'_> {
    fn validate(&self) -> Result<()> {
        check_non_field_attrs(&self.attrs)?;
        if self.attrs.transparent.is_some() {
            if self.fields.len() != 1 {
                return Err(Error::new_spanned(
                    self.original,
                    "#[error(transparent)] requires exactly one field",
                ));
            }
            if let Some(source) = self.fields.iter().find_map(|f| f.attrs.source) {
                return Err(Error::new_spanned(
                    source.original,
                    "transparent variant can't contain #[source]",
                ));
            }
        }
        check_field_attrs(&self.fields)?;
        for field in &self.fields {
            field.validate()?;
        }
        Ok(())
    }
}

impl Field<'_> {
    fn validate(&self) -> Result<()> {
        if let Some(unexpected_display_attr) = if let Some(display) = &self.attrs.display {
            Some(display.original)
        } else if let Some(fmt) = &self.attrs.fmt {
            Some(fmt.original)
        } else {
            None
        } {
            return Err(Error::new_spanned(
                unexpected_display_attr,
                "not expected here; the #[error(...)] attribute belongs on top of a struct or an enum variant",
            ));
        }
        Ok(())
    }
}

fn check_non_field_attrs(attrs: &Attrs) -> Result<()> {
    if let Some(from) = &attrs.from {
        return Err(Error::new_spanned(
            from.original,
            "not expected here; the #[from] attribute belongs on a specific field",
        ));
    }
    if let Some(source) = &attrs.source {
        return Err(Error::new_spanned(
            source.original,
            "not expected here; the #[source] attribute belongs on a specific field",
        ));
    }
    if let Some(backtrace) = &attrs.backtrace {
        return Err(Error::new_spanned(
            backtrace,
            "not expected here; the #[backtrace] attribute belongs on a specific field",
        ));
    }
    if attrs.transparent.is_some() {
        if let Some(display) = &attrs.display {
            return Err(Error::new_spanned(
                display.original,
                "cannot have both #[error(transparent)] and a display attribute",
            ));
        }
        if let Some(fmt) = &attrs.fmt {
            return Err(Error::new_spanned(
                fmt.original,
                "cannot have both #[error(transparent)] and #[error(fmt = ...)]",
            ));
        }
    } else if let (Some(display), Some(_)) = (&attrs.display, &attrs.fmt) {
        return Err(Error::new_spanned(
            display.original,
            "cannot have both #[error(fmt = ...)] and a format arguments attribute",
        ));
    }

    Ok(())
}

fn check_field_attrs(fields: &[Field]) -> Result<()> {
    let mut from_field = None;
    let mut source_field = None;
    let mut backtrace_field = None;
    let mut has_backtrace = false;
    for field in fields {
        if let Some(from) = field.attrs.from {
            if from_field.is_some() {
                return Err(Error::new_spanned(
                    from.original,
                    "duplicate #[from] attribute",
                ));
            }
            from_field = Some(field);
        }
        if let Some(source) = field.attrs.source {
            if source_field.is_some() {
                return Err(Error::new_spanned(
                    source.original,
                    "duplicate #[source] attribute",
                ));
            }
            source_field = Some(field);
        }
        if let Some(backtrace) = field.attrs.backtrace {
            if backtrace_field.is_some() {
                return Err(Error::new_spanned(
                    backtrace,
                    "duplicate #[backtrace] attribute",
                ));
            }
            backtrace_field = Some(field);
            has_backtrace = true;
        }
        if let Some(transparent) = field.attrs.transparent {
            return Err(Error::new_spanned(
                transparent.original,
                "#[error(transparent)] needs to go outside the enum or struct, not on an individual field",
            ));
        }
        has_backtrace |= field.is_backtrace();
    }
    if let (Some(from_field), Some(source_field)) = (from_field, source_field) {
        if from_field.member != source_field.member {
            return Err(Error::new_spanned(
                from_field.attrs.from.unwrap().original,
                "#[from] is only supported on the source field, not any other field",
            ));
        }
    }
    if let Some(from_field) = from_field {
        let max_expected_fields = match backtrace_field {
            Some(backtrace_field) => 1 + (from_field.member != backtrace_field.member) as usize,
            None => 1 + has_backtrace as usize,
        };
        if fields.len() > max_expected_fields {
            return Err(Error::new_spanned(
                from_field.attrs.from.unwrap().original,
                "deriving From requires no fields other than source and backtrace",
            ));
        }
    }
    if let Some(source_field) = source_field.or(from_field) {
        if contains_non_static_lifetime(source_field.ty) {
            return Err(Error::new_spanned(
                &source_field.original.ty,
                "non-static lifetimes are not allowed in the source of an error, because std::error::Error requires the source is dyn Error + 'static",
            ));
        }
    }
    Ok(())
}

fn contains_non_static_lifetime(ty: &Type) -> bool {
    match ty {
        Type::Path(ty) => {
            let bracketed = match &ty.path.segments.last().unwrap().arguments {
                PathArguments::AngleBracketed(bracketed) => bracketed,
                _ => return false,
            };
            for arg in &bracketed.args {
                match arg {
                    GenericArgument::Type(ty) if contains_non_static_lifetime(ty) => return true,
                    GenericArgument::Lifetime(lifetime) if lifetime.ident != "static" => {
                        return true
                    }
                    _ => {}
                }
            }
            false
        }
        Type::Reference(ty) => ty
            .lifetime
            .as_ref()
            .is_some_and(|lifetime| lifetime.ident != "static"),
        _ => false, // maybe implement later if there are common other cases
    }
}

// Two variants that both carry `#[from]` for the same source type would expand
// to two identical `impl From<T> for Error`, which is an E0119 coherence error
// whose span only points at one of the two variants. Detect the directly
// same-syntax case while still inside the derive so that both variants can be
// named in the diagnostic. Type aliases are deliberately not resolved here:
// equality is judged structurally on the tokenized syntax, not by comparing
// path text, so `type IoError = std::io::Error;` slips through and is left to
// the compiler's real coherence check.
fn check_duplicate_from_types(variants: &[Variant]) -> Result<()> {
    let mut seen: Vec<(&Variant, TokenStream)> = Vec::new();
    for variant in variants {
        let Some(from_field) = variant.from_field() else {
            continue;
        };
        // `#[from] Option<T>` generates `impl From<T>` (the Option is wrapped
        // in the constructor), so the comparison is done on the unwrapped type.
        let from_ty =
            crate::expand::type_parameter_of_option(from_field.ty).unwrap_or(from_field.ty);
        let from_tokens = from_ty.to_token_stream();
        if let Some((first_variant, _)) = seen
            .iter()
            .find(|(_, existing_tokens)| existing_tokens.to_string() == from_tokens.to_string())
        {
            // TokenStream's Display renders `::` with surrounding spaces; tidy
            // those up purely for the human-readable message.
            let ty_display = from_tokens.to_string().replace(" :: ", "::");
            let mut error = Error::new_spanned(
                from_field.original,
                format!(
                    "both `{}` and `{}` #[from] fields convert from `{}`, which would create duplicate `From<{}>` implementations",
                    first_variant.ident, variant.ident, ty_display, ty_display,
                ),
            );
            error.combine(Error::new_spanned(
                first_variant.from_field().unwrap().original,
                "first #[from] field with this source type",
            ));
            return Err(error);
        }
        seen.push((variant, from_tokens));
    }
    Ok(())
}
