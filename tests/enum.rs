use cargo_breaking::tests::get_diff;

#[test]
fn not_reported_when_private() {
    let diff = get_diff! {
        {},
        {
            enum A {}
        },
    };

    assert!(diff.is_empty());
}

#[test]
fn new_enum() {
    let diff = get_diff! {
        {},
        {
            pub enum A {}
        },
    };

    assert_eq!(diff.to_string(), "+ A (enum)\n");
}

#[test]
fn new_named_variant_field_is_modification() {
    let diff = get_diff! {
        {
            pub enum A {
                B {}
            }
        },
        {
            pub enum A {
                B {
                    c: u8,
                }
            }
        },
    };

    assert_eq!(diff.to_string(), "+ A::B::c (struct field)\n");
}

#[test]
fn new_unnamed_variant_field_is_modification() {
    let diff = get_diff! {
        {
            pub enum A {
                B()
            }
        },
        {
            pub enum A {
                B(u8)
            }
        },
    };

    assert_eq!(diff.to_string(), "+ A::B::0 (struct field)\n");
}

#[test]
fn named_field_modification() {
    let diff = get_diff! {
        {
            pub enum A {
                B(u8),
            }
        },
        {
            pub enum A {
                B(u16),
            }
        }
    };

    assert_eq!(diff.to_string(), "≠ A::B::0 (struct field)\n");
}
