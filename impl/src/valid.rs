use crate::ast::{Enum, Field, Input, Struct, Variant};
use crate::attr::Attrs;
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
        // Validate each variant independently so that invalid attributes in
        // one variant do not hide diagnostics for the others. The errors are
        // merged in source order into a single diagnostic.
        let mut errors: Option<Error> = None;
        for variant in &self.variants {
            if let Err(error) = variant.validate() {
                accumulate(&mut errors, error);
                // A variant that is already rejected has its invalid
                // attributes diagnosed above; do not derive a second error
                // for the same root cause from the missing-display check.
                continue;
            }
            if has_display
                && variant.attrs.display.is_none()
                && variant.attrs.transparent.is_none()
                && variant.attrs.fmt.is_none()
            {
                accumulate(
                    &mut errors,
                    Error::new_spanned(
                        variant.original,
                        "missing #[error(\"...\")] display attribute",
                    ),
                );
            }
        }
        match errors {
            Some(errors) => Err(errors),
            None => Ok(()),
        }
    }
}

fn accumulate(errors: &mut Option<Error>, error: Error) {
    match errors {
        Some(errors) => errors.combine(error),
        None => *errors = Some(error),
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

#[cfg(test)]
mod tests {
    use crate::ast::Input;
    use proc_macro2::{TokenStream, TokenTree};
    use syn::DeriveInput;

    // Two variants carry independent invalid attributes; the diagnostics
    // must be merged in source order without derived duplicates.
    const TWO_BAD_VARIANTS: &str = "
enum Independent {
    #[error(\"display\")]
    Good,
    #[from]
    First(u32),
    #[source]
    Second(u32),
}
";

    // A single syntax error; parsing must stop there instead of guessing
    // the subsequent structure (the invalid #[from] is never reached).
    const SYNTAX_ERROR: &str = "
enum Broken {
    #[error(\"a\")]
    A
    #[from]
    B(u32),
}
";

    // Valid enum; the generated Display/source/From impls must be unchanged.
    const VALID_ENUM: &str = "
enum Valid {
    #[error(\"io error\")]
    Io(#[from] std::io::Error),
    #[error(\"message: {0}\")]
    Message(String),
}
";

    // Mirror of the derive entry point: a syntax error is returned directly
    // (parse_macro_input! behavior), otherwise expansion runs.
    fn derive(source: &str) -> TokenStream {
        match syn::parse_str::<DeriveInput>(source) {
            Ok(ast) => crate::expand::derive(&ast),
            Err(error) => error.into_compile_error(),
        }
    }

    fn validate(source: &str) -> Result<(), syn::Error> {
        let ast: DeriveInput = syn::parse_str(source).unwrap();
        Input::from_syn(&ast)?.validate()
    }

    // Every diagnostic emitted by an error as (message, line, column), in
    // emission order. Lines are 1-based, columns 0-based.
    fn compile_errors(error: &syn::Error) -> Vec<(String, usize, usize)> {
        compile_errors_in(error.to_compile_error())
    }

    fn compile_errors_in(tokens: TokenStream) -> Vec<(String, usize, usize)> {
        let mut errors = Vec::new();
        let mut iter = tokens.into_iter();
        while let Some(tt) = iter.next() {
            let ident = match &tt {
                TokenTree::Ident(ident) if ident == "compile_error" => ident,
                _ => continue,
            };
            let start = ident.span().start();
            let message = iter
                .find_map(|tt| match tt {
                    TokenTree::Group(group) => match group.stream().into_iter().next() {
                        Some(TokenTree::Literal(lit)) => Some(lit.to_string()),
                        _ => None,
                    },
                    _ => None,
                })
                .expect("compile_error! message");
            errors.push((message, start.line, start.column));
        }
        errors
    }

    #[test]
    fn two_variants_independent_errors() {
        let error = validate(TWO_BAD_VARIANTS).unwrap_err();
        let errors = compile_errors(&error);

        // One diagnostic per invalid variant, merged in source order; the
        // missing-display check must not derive duplicates for variants
        // that are already rejected (same root cause).
        assert_eq!(errors.len(), 2, "expected exactly two diagnostics: {errors:?}");
        assert!(
            errors[0].0.contains("#[from] attribute belongs on a specific field"),
            "first diagnostic: {}",
            errors[0].0,
        );
        assert!(
            errors[1].0.contains("#[source] attribute belongs on a specific field"),
            "second diagnostic: {}",
            errors[1].0,
        );

        // Positions point at the offending attributes, in source order.
        assert_eq!((errors[0].1, errors[0].2), (5, 4), "#[from] attribute");
        assert_eq!((errors[1].1, errors[1].2), (7, 4), "#[source] attribute");
        assert!(errors[0].1 < errors[1].1, "diagnostics kept in source order");

        // Both diagnostics survive the fallback path of the full derive.
        let expanded = derive(TWO_BAD_VARIANTS).to_string();
        assert_eq!(expanded.matches("compile_error").count(), 2, "{expanded}");
    }

    #[test]
    fn single_syntax_error_stops_parsing() {
        let error = match syn::parse_str::<DeriveInput>(SYNTAX_ERROR) {
            Ok(_ast) => panic!("expected a syntax error"),
            Err(error) => error,
        };
        let errors = compile_errors(&error);

        // Exactly one parse diagnostic; the parser does not guess the
        // subsequent structure, so the invalid #[from] on the next variant
        // is never diagnosed.
        assert_eq!(errors.len(), 1, "expected exactly one diagnostic: {errors:?}");
        assert!(errors[0].0.contains("expected"), "parse error: {}", errors[0].0);
        assert!(!errors[0].0.contains("#[from]"), "no derived error: {}", errors[0].0);

        // The derive entry returns the parse error directly without
        // emitting fallback impls for a type it could not parse.
        let expanded = derive(SYNTAX_ERROR).to_string();
        assert_eq!(expanded.matches("compile_error").count(), 1, "{expanded}");
        assert!(!expanded.contains("impl"), "no fallback impls: {expanded}");
    }

    #[test]
    fn valid_enum_impls_unchanged() {
        validate(VALID_ENUM).unwrap();

        let expanded = derive(VALID_ENUM).to_string();
        assert!(!expanded.contains("compile_error"), "{expanded}");
        assert!(expanded.contains("Display"), "Display impl: {expanded}");
        assert!(expanded.contains("fn source"), "source(): {expanded}");
        assert!(expanded.contains("From"), "From impl: {expanded}");
    }
}
