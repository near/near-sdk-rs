use darling::util::PathList;
use darling::FromMeta;

/// Holds configuration of macros.
#[derive(Default, FromMeta, Clone, Debug)]
pub struct MacroConfig {
    /// Prevents attributes on methods of a `Contract` to be forwarded to
    /// automatically generated methods of `ContractExt`.
    ///
    /// To be used with `#[near_bindgen]` on implementation blocks that contain
    /// methods whose attributes should not be forwarded.
    ///
    /// # Macros supporting this parameter
    ///
    /// - near_bindgen
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Attributes `attr_1` and `attr_2` will not be forwarded to
    /// // `ContractExt::foo`.
    /// #[near_bindgen(blacklist_ext_fn_attrs(attr1, attr2))]
    /// impl Contract {
    ///     #[attr_1]
    ///     #[attr_2]
    ///     pub fn foo(&self) {}
    /// }
    /// ```
    pub blacklist_ext_fn_attrs: Option<PathList>,
}
