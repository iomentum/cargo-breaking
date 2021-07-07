use cargo_breaking::ApiCompatibilityDiagnostics;
use syn::parse_quote;

#[test]
fn private_is_not_reported() {
    let diff: ApiCompatibilityDiagnostics = parse_quote! {
        {},
        {
            struct A;
        },
    };

    assert!(diff.is_empty());
}

#[test]
fn addition() {
    let diff: ApiCompatibilityDiagnostics = parse_quote! {
        {},
        {
            pub struct A;
        }
    };

    assert_eq!(diff.to_string(), "+ A\n");
}

#[test]
fn removal() {
    let diff: ApiCompatibilityDiagnostics = parse_quote! {
        {
            pub struct B;
        },
        {}
    };

    assert_eq!(diff.to_string(), "- B\n");
}

#[test]
fn new_public_field_tupled_is_modification() {
    let diff: ApiCompatibilityDiagnostics = parse_quote! {
        {
            pub struct C;
        },
        {
            pub struct C(pub u8);
        },
    };

    assert_eq!(diff.to_string(), "≠ C\n");
}

#[test]
fn new_private_field_tupled_is_modification() {
    let diff: ApiCompatibilityDiagnostics = parse_quote! {
        {
            pub struct C();
        },
        {
            pub struct C(usize);
        },
    };

    assert!(diff.is_empty());
}

#[test]
fn new_public_field_named_is_modification() {
    let diff: ApiCompatibilityDiagnostics = parse_quote! {
        {
            pub struct D {}
        },
        {
            pub struct D {
                pub a: u8,
            }
        },
    };

    assert_eq!(diff.to_string(), "≠ D\n");
}

#[test]
fn new_private_field_named_is_not_reported() {
    let diff: ApiCompatibilityDiagnostics = parse_quote! {
        {
            pub struct D {
                a: b,
            }
        },
        {
            pub struct D {
                a: b,
                c: d,
            }
        },
    };

    assert!(diff.is_empty());
}

#[test]
fn public_named_field_modification() {
    let diff: ApiCompatibilityDiagnostics = parse_quote! {
        {
            pub struct A {
                pub a: u8,
            }
        },
        {
            pub struct A {
                pub a: u16,
            }
        },
    };

    assert_eq!(diff.to_string(), "≠ A\n");
}

#[test]
fn public_unnamed_field_modification() {
    let diff: ApiCompatibilityDiagnostics = parse_quote! {
        {
            pub struct A(pub u8);
        },
        {
            pub struct A(pub u16);
        },
    };

    assert_eq!(diff.to_string(), "≠ A\n");
}

#[test]
fn public_named_field_removal_is_modification() {
    let diff: ApiCompatibilityDiagnostics = parse_quote! {
        {
            pub struct A {
                pub a: u8,
            }
        },
        {
            pub struct A {}
        },
    };

    assert_eq!(diff.to_string(), "≠ A\n");
}

#[test]
fn public_unnamed_field_removal_is_modification() {
    let diff: ApiCompatibilityDiagnostics = parse_quote! {
        {
            pub struct A(pub u8);
        },
        {
            pub struct A();
        },
    };

    assert_eq!(diff.to_string(), "≠ A\n");
}

#[test]
fn generic_change_is_modification() {
    let diff: ApiCompatibilityDiagnostics = parse_quote! {
        {
            pub struct E;
        },
        {
            pub struct E<T>;
        },
    };

    assert_eq!(diff.to_string(), "≠ E\n");
}
