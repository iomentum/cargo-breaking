use cargo_breaking::ApiCompatibilityDiagnostics;
use syn::parse_quote;

#[test]
fn private_is_not_reported() {
    let diff: ApiCompatibilityDiagnostics = parse_quote! {
        {},
        {
            fn fact(n: u32) -> u32 {}
        },
    };

    assert!(diff.is_empty());
}

#[test]
fn addition() {
    let diff: ApiCompatibilityDiagnostics = parse_quote! {
        {},
        {
            pub fn fact(n: u32) -> u32 {}
        },
    };

    assert_eq!(diff.to_string(), "+ fact\n");
}

#[test]
fn new_arg() {
    let diff: ApiCompatibilityDiagnostics = parse_quote! {
        {
            pub fn fact() {}
        },
        {
            pub fn fact(n: u32) {}
        }
    };

    assert_eq!(diff.to_string(), "≠ fact\n");
}

#[test]
fn generic_order() {
    let diff: ApiCompatibilityDiagnostics = parse_quote! {
        {
            pub fn f<T, E>() {}
        },
        {
            pub fn f<E, T>() {}
        },
    };

    assert_eq!(diff.to_string(), "≠ f\n");
}

#[test]
fn body_change_not_detected() {
    let diff: ApiCompatibilityDiagnostics = parse_quote! {
        {
            pub fn fact() {}
        },
        {
            pub fn fact() { todo!() }
        },
    };

    assert!(diff.is_empty());
}

#[test]
fn fn_arg_comma_is_removed() {
    let diff: ApiCompatibilityDiagnostics = parse_quote! {
        {
            pub fn a(a: t, b: t, c: t,) {}
        },
        {
            pub fn a(a: t, b: t, c: t) {}
        },
    };

    assert!(diff.is_empty());
}

#[test]
fn fn_arg_last_character_not_removed() {
    let diff: ApiCompatibilityDiagnostics = parse_quote! {
        {
            pub fn a(a: t, b: t, c: t) {}
        },
        {
            pub fn a(a: t, b: t, c: u) {}
        },
    };

    assert_eq!(diff.to_string(), "≠ a\n");
}

#[test]
fn empty_struct_kind_change_is_modification() {
    let files = ["pub struct A;", "pub struct A();", "pub struct A {}"];

    for (id_a, file_a) in files.iter().enumerate() {
        for (id_b, file_b) in files.iter().enumerate() {
            let comparator = cargo_breaking::compare(file_a, file_b).unwrap();
            let diff = comparator.run();

            if id_a != id_b {
                assert_eq!(diff.to_string(), "≠ A\n");
            } else {
                assert!(diff.is_empty());
            }
        }
    }
}
