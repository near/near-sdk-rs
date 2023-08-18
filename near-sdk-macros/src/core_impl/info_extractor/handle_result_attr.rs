use syn::spanned::Spanned as _;
use syn::{Attribute, Meta, NestedMeta};

pub struct HandleResultAttr {
    pub check: bool,
}

impl HandleResultAttr {
    pub fn from_attr(attr: &Attribute) -> syn::Result<Self> {
        let meta = attr.parse_meta()?;

        let mut check = true;

        match meta {
            Meta::Path(_) => {}
            Meta::List(l) => {
                for meta in l.nested {
                    let span = meta.span();
                    let err = || Err(syn::Error::new(span, "invalid attribute"));

                    match meta {
                        NestedMeta::Meta(Meta::Path(p)) => {
                            if let Some(ident) = p.get_ident() {
                                match ident.to_string().as_str() {
                                    "aliased" => check = false,
                                    _ => return err(),
                                }
                            } else {
                                return err();
                            }
                        }
                        _ => return err(),
                    }
                }
            }
            Meta::NameValue(_) => {
                return Err(syn::Error::new(attr.span(), "unexpected name-value pair"))
            }
        };

        Ok(Self { check })
    }
}
