use super::visitor::Visitor;
use super::{ArgInfo, BindgenArgType, InitAttr, MethodKind, SerializerAttr, SerializerType};
use crate::core_impl::{utils, Returns};
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{Attribute, Error, FnArg, GenericParam, Ident, ReturnType, Signature, Type};

/// Information extracted from method attributes and signature.
pub struct AttrSigInfo {
    /// The name of the method.
    pub ident: Ident,
    /// Attributes not related to bindgen.
    pub non_bindgen_attrs: Vec<Attribute>,
    /// All arguments of the method.
    pub args: Vec<ArgInfo>,
    /// Describes the type of the method.
    pub method_kind: MethodKind,
    /// What this function returns.
    pub returns: Returns,
    /// The serializer that we use for `env::input()`.
    pub input_serializer: SerializerType,
    /// The original method signature.
    pub original_sig: Signature,
}

impl AttrSigInfo {
    /// Apart from replacing `Self` types with their concretions, returns spans of all `Self` tokens found.
    fn sanitize_self(
        original_sig: &mut Signature,
        source_type: &TokenStream2,
    ) -> syn::Result<Vec<Span>> {
        match original_sig.output {
            ReturnType::Default => Ok(vec![]),
            ReturnType::Type(_, ref mut ty) => match ty.as_mut() {
                x @ (Type::Array(_) | Type::Path(_) | Type::Tuple(_) | Type::Group(_)) => {
                    let res = utils::sanitize_self(x, source_type)?;
                    *ty = res.ty.into();
                    Ok(res.self_occurrences)
                }
                Type::Reference(ref mut r) => {
                    let res = utils::sanitize_self(&r.elem, source_type)?;
                    r.elem = res.ty.into();
                    Ok(res.self_occurrences)
                }
                _ => Err(Error::new(ty.span(), "Unsupported contract API type.")),
            },
        }
    }

    pub fn new(
        original_attrs: &mut Vec<Attribute>,
        original_sig: &mut Signature,
        source_type: &TokenStream2,
    ) -> syn::Result<Self> {
        let mut self_occurrences = Self::sanitize_self(original_sig, source_type)?;

        let mut errors = vec![];
        for generic in &original_sig.generics.params {
            match generic {
                GenericParam::Type(type_generic) => {
                    errors.push(Error::new(
                        type_generic.span(),
                        "Contract API is not allowed to have generics.",
                    ));
                }
                GenericParam::Const(const_generic) => {
                    // `generic.span()` points to the `const` part of const generics, so we use `ident` explicitly.
                    errors.push(Error::new(
                        const_generic.ident.span(),
                        "Contract API is not allowed to have generics.",
                    ));
                }
                _ => {}
            }
        }
        if let Some(combined_errors) = errors.into_iter().reduce(|mut l, r| (l.combine(r), l).1) {
            return Err(combined_errors);
        }

        let mut visitor = Visitor::new(original_attrs, original_sig);

        let ident = original_sig.ident.clone();
        let mut non_bindgen_attrs = vec![];

        // Visit attributes
        for attr in original_attrs.iter() {
            let attr_str = attr.path.to_token_stream().to_string();
            match attr_str.as_str() {
                "init" => {
                    let init_attr: InitAttr = syn::parse2(attr.tokens.clone())?;
                    visitor.visit_init_attr(attr, &init_attr)?;
                }
                "payable" => {
                    visitor.visit_payable_attr(attr)?;
                }
                "private" => {
                    visitor.visit_private_attr(attr)?;
                }
                "result_serializer" => {
                    let serializer: SerializerAttr = syn::parse2(attr.tokens.clone())?;
                    visitor.visit_result_serializer_attr(attr, &serializer)?;
                }
                "handle_result" => {
                    visitor.visit_handle_result_attr();
                }
                _ => {
                    non_bindgen_attrs.push((*attr).clone());
                }
            }
        }

        // Visit arguments
        let mut args = vec![];
        for fn_arg in &mut original_sig.inputs {
            match fn_arg {
                FnArg::Receiver(r) => visitor.visit_receiver(r)?,
                FnArg::Typed(pat_typed) => {
                    args.push(ArgInfo::new(pat_typed, source_type)?);
                }
            }
        }

        let (method_kind, returns) = visitor.build()?;

        self_occurrences.extend(args.iter().flat_map(|arg| arg.self_occurrences.clone()));

        *original_attrs = non_bindgen_attrs.clone();

        if !self_occurrences.is_empty()
            && matches!(method_kind, MethodKind::Call(_) | MethodKind::View(_))
        {
            // TODO: return an error instead in 5.0
            // see https://github.com/near/near-sdk-rs/issues/1005
            println!(
                "near_bindgen: references to `Self` in non-init methods will be forbidden in 5.0"
            );

            // Once proc_macro::Diagnostic is stabilized, we could start getting rid of the `println` and
            // try the code below. See: https://github.com/rust-lang/rust/issues/54140
            //
            // proc_macro::Diagnostic::spanned(
            //     self_occurrences.into(),
            //     proc_macro::Level::Warning,
            //     "references to `Self` in non-init methods will be forbidden in 5.0",
            // )
            // .emit();
            //
        }

        let mut result = AttrSigInfo {
            ident,
            non_bindgen_attrs,
            args,
            method_kind,
            returns,
            input_serializer: SerializerType::JSON,
            original_sig: original_sig.clone(),
        };

        let input_serializer =
            if result.input_args().all(|arg: &ArgInfo| arg.serializer_ty == SerializerType::JSON) {
                SerializerType::JSON
            } else if result.input_args().all(|arg| arg.serializer_ty == SerializerType::Borsh) {
                SerializerType::Borsh
            } else {
                return Err(Error::new(
                    Span::call_site(),
                    "Input arguments should be all of the same serialization type.",
                ));
            };
        result.input_serializer = input_serializer;
        Ok(result)
    }

    /// Only get args that correspond to `env::input()`.
    pub fn input_args(&self) -> impl Iterator<Item = &ArgInfo> {
        self.args.iter().filter(|arg| matches!(arg.bindgen_ty, BindgenArgType::Regular))
    }
}
