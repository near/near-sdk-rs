use syn::{Attribute, Lit::Str, Meta::NameValue, MetaNameValue};

pub fn parse_rustdoc(attrs: &[Attribute]) -> Option<String> {
    let doc = attrs
        .iter()
        .filter_map(|attr| {
            if attr.path.is_ident("doc") {
                if let NameValue(MetaNameValue { lit: Str(s), .. }) = attr.parse_meta().ok()? {
                    Some(s.value())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    if doc.is_empty() {
        None
    } else {
        Some(doc)
    }
}
