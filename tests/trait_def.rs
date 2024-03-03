use cargo_breaking::tests::get_diff;

#[test]
fn addition_simple() {
    let diff = get_diff! {
        {},
        {
            pub trait A {}
        }
    };

    assert_eq!(diff.to_string(), "+ A (trait)\n");
}

#[test]
fn trait_item_addition() {
    let diff = get_diff! {
        {
            pub trait A {}
        },
        {
            pub trait A { type B; }
        },
    };

    assert_eq!(diff.to_string(), "+ A::B (associated type)\n");
}

#[test]
fn trait_item_modification() {
    let diff = get_diff! {
        {
            pub trait A {
                const B: u8 = 5;
            }
        },
        {
            pub trait A {
                const B: u8 = 6;
            }
        },
    };

    assert_eq!(diff.to_string(), "≠ A::B (associated constant)\n");
}

#[test]
fn trait_item_removal() {
    let diff = get_diff! {
        {
            pub trait A {
                type B;
            }
        },
        {
            pub trait A {}
        },
    };

    assert_eq!(diff.to_string(), "- A::B (associated type)\n");
}

#[test]
fn trait_item_kind_modification() {
    let diff = get_diff! {
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

    assert_eq!(
        diff.to_string(),
        "- A::B (associated type)\n+ A::B (associated constant)\n"
    );
}

#[test]
fn in_private_module() {
    let diff = get_diff! {
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
    let diff = get_diff! {
        {
            pub mod a {}
        },
        {
            pub mod a {
                pub trait A {}
            }
        },
    };

    assert_eq!(diff.to_string(), "+ a::A (trait)\n");
}
