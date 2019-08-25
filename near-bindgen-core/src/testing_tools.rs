use syn::export::TokenStream2;

fn prettify_code(input: String) -> String {
    let mut buf = Vec::new();
    {
        let mut config = rustfmt_nightly::Config::default();
        config.set().emit_mode(rustfmt_nightly::EmitMode::Stdout);
        config.set().edition(rustfmt_nightly::Edition::Edition2018);
        let mut session = rustfmt_nightly::Session::new(config, Some(&mut buf));
        session.format(rustfmt_nightly::Input::Text(input)).unwrap();
    }
    String::from_utf8(buf).unwrap()
}

pub fn assert_eq(expected: TokenStream2, actual: TokenStream2) {
    let expected = prettify_code(expected.to_string());
    let actual = prettify_code(actual.to_string());
    if expected != actual {
        println!("{}", expected);
        println!("{}", actual);
        panic!("FAILED");
    }
//    pretty_assertions::assert_eq!(expected, actual);
}
