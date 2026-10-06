use crate::ast::{Enum, Field, Input, Struct, Variant};
use crate::attr::Attrs;
use syn::{Attribute, Error, GenericArgument, PathArguments, Result, Type};

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
            check_transparent_field_attrs(&self.fields[0], "transparent error struct")?;
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
            check_transparent_field_attrs(&self.fields[0], "transparent variant")?;
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

// The single field of a transparent struct or variant is implicitly the
// source of the error, so an explicit #[source] or #[backtrace] attribute on
// that field would make the generated Error impl ambiguous. Diagnose every
// conflicting attribute, each at its own span, instead of just the first one
// found. This is a purely static check of the attribute contract: it never
// requires executing Display or constructing an error value.
fn check_transparent_field_attrs(field: &Field, container: &str) -> Result<()> {
    let mut errors = None;
    if let Some(source) = field.attrs.source {
        combine_error(
            &mut errors,
            Error::new_spanned(
                source.original,
                format!("{container} can't contain #[source]"),
            ),
        );
    }
    if let Some(backtrace) = field.attrs.backtrace {
        combine_error(
            &mut errors,
            Error::new_spanned(backtrace, format!("{container} can't contain #[backtrace]")),
        );
    }
    match errors {
        Some(errors) => Err(errors),
        None => Ok(()),
    }
}

// Report a duplicated field-level attribute at the span of every conflicting
// occurrence: the duplicate itself, plus the first occurrence it conflicts
// with. Fields are validated per struct or per variant, so these spans never
// point into a different variant than the one containing the duplicate.
fn duplicate_attr_error(attr: &str, first: &Attribute, duplicate: &Attribute) -> Error {
    let mut error = Error::new_spanned(duplicate, format!("duplicate #[{attr}] attribute"));
    error.combine(Error::new_spanned(
        first,
        format!("first #[{attr}] attribute is here"),
    ));
    error
}

fn combine_error(errors: &mut Option<Error>, new: Error) {
    match errors {
        Some(errors) => errors.combine(new),
        None => *errors = Some(new),
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
    let mut from_field: Option<&Field> = None;
    let mut source_field: Option<&Field> = None;
    let mut backtrace_field: Option<&Field> = None;
    let mut has_backtrace = false;
    let mut errors: Option<Error> = None;
    for field in fields {
        if let Some(from) = field.attrs.from {
            if let Some(first) = from_field {
                let first = first.attrs.from.unwrap();
                combine_error(
                    &mut errors,
                    duplicate_attr_error("from", first.original, from.original),
                );
            } else {
                from_field = Some(field);
            }
        }
        if let Some(source) = field.attrs.source {
            if let Some(first) = source_field {
                let first = first.attrs.source.unwrap();
                combine_error(
                    &mut errors,
                    duplicate_attr_error("source", first.original, source.original),
                );
            } else {
                source_field = Some(field);
            }
        }
        if let Some(backtrace) = field.attrs.backtrace {
            if let Some(first) = backtrace_field {
                combine_error(
                    &mut errors,
                    duplicate_attr_error("backtrace", first.attrs.backtrace.unwrap(), backtrace),
                );
            } else {
                backtrace_field = Some(field);
            }
            has_backtrace = true;
        }
        if let Some(transparent) = field.attrs.transparent {
            combine_error(
                &mut errors,
                Error::new_spanned(
                    transparent.original,
                    "#[error(transparent)] needs to go outside the enum or struct, not on an individual field",
                ),
            );
        }
        has_backtrace |= field.is_backtrace();
    }
    if let Some(errors) = errors {
        return Err(errors);
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
