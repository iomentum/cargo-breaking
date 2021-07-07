use cargo_breaking::ApiCompatibilityDiagnostics;
use syn::parse_quote;

#[test]
fn not_reported_when_private() {
    let diff: ApiCompatibilityDiagnostics = parse_quote! {
        {},
        {
            enum A {}
        },
    };

    assert!(diff.is_empty());
}

#[test]
fn new_enum() {
    let diff: ApiCompatibilityDiagnostics = parse_quote! {
        {},
        {
            pub enum A {}
        },
    };

    assert_eq!(diff.to_string(), "+ A\n");
}

#[test]
fn new_named_variant_field_is_modification() {
    let diff: ApiCompatibilityDiagnostics = parse_quote! {
        {
            pub enum A {
                B {}
            }
        },
        {
            pub enum A {
                B {
                    pub c: u8,
                }
            }
        },
    };

    assert_eq!(diff.to_string(), "≠ A\n");
}

#[test]
fn new_unnamed_variant_field_is_modification() {
    let diff: ApiCompatibilityDiagnostics = parse_quote! {
        {
            pub enum A {
                B() }
        },
        {
            pub enum A {
                B(pub u8)
            }
        },
    };

    assert_eq!(diff.to_string(), "≠ A\n");
}

#[test]
fn named_field_modification() {
    let diff: ApiCompatibilityDiagnostics = parse_quote! {
        {
            pub enum A {
                B(pub u8),
            }
        },
        {
            pub enum A {
                B(pub u16),
            }
        }
    };

    assert_eq!(diff.to_string(), "≠ A\n");
}

#[test]
fn empty_variant_kind_change_is_modification() {
    let files = [
        "pub enum A { B }",
        "pub enum A { B() }",
        "pub enum A { B {} }",
    ];

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
