use syn::{AttributeArgs, NestedMeta, Meta, Expr, Lit};

/// Parse `crate = "name"` from macro attribute args.
/// Falls back to `near_sdk` when not specified.
/// Hyphens are converted to underscores to produce a valid Rust identifier.
pub fn parse_crate_name(args: &AttributeArgs) -> String {
    for arg in args {
        if let NestedMeta::Meta(Meta::NameValue(nv)) = arg {
            if nv.path.is_ident("crate") {
                if let Expr::Lit(expr_lit) = &nv.value {
                    if let Lit::Str(lit_str) = &expr_lit.lit {
                        let s = lit_str.value();
                        return s.replace('-', "_");
                    }
                }
            }
        }
    }
    "near_sdk".to_string()
}
