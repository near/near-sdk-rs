wit_bindgen::generate!({
    ownership: Borrowing { duplicate_if_necessary: true },
    generate_all,
    additional_derives: [PartialEq, Eq],
});
