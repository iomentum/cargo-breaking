use cargo_breaking::ApiCompatibilityDiagnostics;
use syn::parse_quote;

#[test]
fn addition_simple() {
    let diff: ApiCompatibilityDiagnostics = parse_quote! {
        {},
        {
            pub trait A {}
        }
    };

    assert_eq!(diff.to_string(), "+ A\n");
}

#[test]
fn trait_item_addition() {
    let diff: ApiCompatibilityDiagnostics = parse_quote! {
        {
            pub trait A {}
        },
        {
            pub trait A { type B; }
        },
    };

    assert_eq!(diff.to_string(), "+ A::B\n");
}

#[test]
fn trait_item_modification() {
    let diff: ApiCompatibilityDiagnostics = parse_quote! {
        {
            pub trait A {
                type B = u8;
            }
        },
        {
            pub trait A {
                type B = u16;
            }
        },
    };

    assert_eq!(diff.to_string(), "≠ A::B\n");
}

#[test]
fn trait_item_removal() {
    let diff: ApiCompatibilityDiagnostics = parse_quote! {
        {
            pub trait A {
                type B;
            }
        },
        {
            pub trait A {}
        },
    };

    assert_eq!(diff.to_string(), "- A::B\n");
}

#[test]
fn trait_item_kind_modification() {
    let diff: ApiCompatibilityDiagnostics = parse_quote! {
        {
            pub trait A {
                type B;
            }
        },
        {
            pub trait A {
                const B: usize;
            }
        },
    };

    assert_eq!(diff.to_string(), "- A::B\n+ A::B\n");
}

#[test]
fn in_private_module() {
    let diff: ApiCompatibilityDiagnostics = parse_quote! {
        {},
        {
            mod a {
                pub trait A {}
            }
        },
    };

    assert!(diff.is_empty());
}

#[test]
fn in_public_module() {
    let diff: ApiCompatibilityDiagnostics = parse_quote! {
        {
            pub mod a {}
        },
        {
            pub mod a {
                pub trait A {}
            }
        },
    };

    assert_eq!(diff.to_string(), "+ a::A\n");
}
