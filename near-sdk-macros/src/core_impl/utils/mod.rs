use syn::{GenericArgument, Path, PathArguments, Type};

/// Checks whether the given path ends in "Result".
/// Note that it won't match a type alias like `type StringResult = Result<String, String>`.
pub(crate) fn path_is_result(path: &Path) -> bool {
    path.segments.last().map_or_else(|| false, |last_segment| last_segment.ident == "Result")
}

/// Equivalent to `path_is_result` except that it works on `Type` values.
pub(crate) fn type_is_result(ty: &Type) -> bool {
    match ty {
        Type::Path(type_path) if type_path.qself.is_none() => path_is_result(&type_path.path),
        _ => false,
    }
}

/// Extracts the Ok type from a `Result` type.
///
/// For example, given `Result<String, u8>` type it will return `String` type.
pub(crate) fn extract_ok_type(ty: &Type) -> Option<&Type> {
    match ty {
        Type::Path(type_path) if type_path.qself.is_none() && path_is_result(&type_path.path) => {
            // Get the last segment of the path ("Result"):
            let type_params = &type_path.path.segments.last()?.arguments;
            // We are interested in the first angle-bracketed param responsible for Ok type ("<OkType, ErrType>" or "<OkType>"):
            let generic_arg = match type_params {
                PathArguments::AngleBracketed(params) => Some(params.args.first()?),
                _ => None,
            }?;
            // This argument must be a type:
            match generic_arg {
                GenericArgument::Type(ty) => Some(ty),
                _ => None,
            }
        }
        _ => None,
    }
}
