use syn::{GenericArgument, Path, PathArguments, PathSegment, Type};

pub(crate) enum ReturnVariant {
    PromiseOrValue,
    Promise,
    Value,
}

pub(crate) fn function_return_variant(ty: &Type) -> ReturnVariant {
    if let Some(last) = last_type_segment(ty) {
        if last.ident == "PromiseOrValue" {
            return ReturnVariant::PromiseOrValue;
        } else if last.ident == "ScheduledFn" {
            return ReturnVariant::Promise;
        }
    }

    ReturnVariant::Value
}

fn first_generic_type(segment: &PathSegment) -> Option<&Type> {
    if let PathArguments::AngleBracketed(params) = &segment.arguments {
        if let Some(GenericArgument::Type(inner)) = params.args.first() {
            return Some(inner);
        }
    }
    None
}

fn last_type_segment(ty: &Type) -> Option<&PathSegment> {
    if let Type::Path(type_path) = ty {
        type_path.path.segments.last()
    } else {
        None
    }
}

pub(crate) fn remove_promise_types_recursively(mut ty: &Type) -> &Type {
    loop {
        if let Some(last) = last_type_segment(ty) {
            // TODO add others
            if last.ident == "PromiseOrValue" || last.ident == "ScheduledFn" {
                if let Some(inner) = first_generic_type(last) {
                    ty = inner;
                    continue;
                }
            }
        }
        break;
    }

    ty
}

/// Checks whether the given path is literally "Result".
/// Note that it won't match a fully qualified name `core::result::Result` or a type alias like
/// `type StringResult = Result<String, String>`.
pub(crate) fn path_is_result(path: &Path) -> bool {
    path.leading_colon.is_none()
        && path.segments.len() == 1
        && path.segments.iter().next().unwrap().ident == "Result"
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
            // Get the first segment of the path (there should be only one, in fact: "Result"):
            let type_params = &type_path.path.segments.first()?.arguments;
            // We are interested in the first angle-bracketed param responsible for Ok type ("<String, _>"):
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

/// Checks whether the given path is literally "Vec".
/// Note that it won't match a fully qualified name `std::vec::Vec` or a type alias like
/// `type MyVec = Vec<String>`.
#[cfg(feature = "__abi-generate")]
fn path_is_vec(path: &Path) -> bool {
    path.leading_colon.is_none()
        && path.segments.len() == 1
        && path.segments.iter().next().unwrap().ident == "Vec"
}

/// Extracts the inner generic type from a `Vec<_>` type.
///
/// For example, given `Vec<String>` this function will return `String`.
#[cfg(feature = "__abi-generate")]
pub(crate) fn extract_vec_type(ty: &Type) -> Option<&Type> {
    match ty {
        Type::Path(type_path) if type_path.qself.is_none() && path_is_vec(&type_path.path) => {
            let type_params = &type_path.path.segments.first()?.arguments;
            let generic_arg = match type_params {
                // We are interested in the first (and only) angle-bracketed param:
                PathArguments::AngleBracketed(params) if params.args.len() == 1 => {
                    Some(params.args.first()?)
                }
                _ => None,
            }?;
            match generic_arg {
                GenericArgument::Type(ty) => Some(ty),
                _ => None,
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::ToTokens;
    use syn::parse_quote;

    #[test]
    fn extract_types_test() {
        macro_rules! assert_conversion {
            ($fn:ident($($init:tt)*), $($res:tt)*) => {
                let init: Type = parse_quote!($($init)*);
                let res: Type = parse_quote!($($res)*);
                assert_eq!($fn(&init).into_token_stream().to_string(), res.into_token_stream().to_string());
            };
        }

        assert_conversion!(
            remove_promise_types_recursively(near_sdk::promise::PromiseOrValue<String>),
            String
        );
        assert_conversion!(remove_promise_types_recursively(Result<String>), Result<String>);
        assert_conversion!(remove_promise_types_recursively(ScheduledFn<String>), String);
        assert_conversion!(remove_promise_types_recursively(u8), u8);
        assert_conversion!(
            remove_promise_types_recursively(ScheduledFn<PromiseOrValue<String>>),
            String
        );
        // TODO do we want to skip over wrapper types?
        // assert_conversion!(remove_promise_types_recursively(Option<PromiseOrValue<String>>), Option<String>);
    }
}
