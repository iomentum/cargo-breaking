use cargo_breaking::tests::get_diff;

#[test]
fn private_is_not_reported() {
    let diff = get_diff! {
        {},
        {
            struct A;
        },
    };

    assert!(diff.is_empty());
}

#[test]
fn addition() {
    let diff = get_diff! {
        {},
        {
            pub struct A;
        }
    };

    assert_eq!(diff.to_string(), "+ A (struct)\n");
}

#[test]
fn removal() {
    let diff = get_diff! {
        {
            pub struct B;
        },
        {}
    };

    assert_eq!(diff.to_string(), "- B (struct)\n");
}

#[test]
fn new_public_field_tupled_is_modification() {
    let diff = get_diff! {
        {
            pub struct C;
        },
        {
            pub struct C(pub u8);
        },
    };

    assert_eq!(diff.to_string(), "≠ C (struct)\n+ C::0 (struct field)\n");
}

#[test]
fn new_private_field_tupled_is_modification() {
    let diff = get_diff! {
        {
            pub struct C();
        },
        {
            pub struct C(usize);
        },
    };

    assert_eq!(diff.to_string(), "≠ C (struct)\n");
}

#[test]
fn remove_private_field_tupled_is_not_modification() {
    let diff = get_diff! {
        {
            pub struct C(usize);
        },
        {
            pub struct C();
        },
    };

    assert!(diff.is_empty());
}

#[test]
fn new_public_field_named_is_modification() {
    let diff = get_diff! {
        {
            pub struct D {}
        },
        {
            pub struct D {
                pub a: u8,
            }
        },
    };

    assert_eq!(diff.to_string(), "+ D::a (struct field)\n");
}

#[test]
fn new_private_field_named_is_not_reported() {
    let diff = get_diff! {
        {
            pub struct D {
                a: u8,
            }
        },
        {
            pub struct D {
                a: u8,
                c: u8,
            }
        },
    };

    assert!(diff.is_empty());
}

#[test]
fn new_private_field_named_with_existing_public_is_modification() {
    let diff = get_diff! {
        {
            pub struct D {
                pub a: u8,
            }
        },
        {
            pub struct D {
                pub a: u8,
                c: u8,
            }
        },
    };

    assert_eq!(diff.to_string(), "≠ D (struct)\n");
}

#[test]
fn remove_private_field_named_with_existing_public_is_modification() {
    let diff = get_diff! {
        {
            pub struct D {
                pub a: u8,
                c: u8,
            }
        },
        {
            pub struct D {
                pub a: u8,
            }
        },
    };

    assert!(diff.is_empty());
}

#[test]
fn public_named_field_modification() {
    let diff = get_diff! {
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

    assert_eq!(diff.to_string(), "≠ A::a (struct field)\n");
}

#[test]
fn public_unnamed_field_modification() {
    let diff = get_diff! {
        {
            pub struct A(pub u8);
        },
        {
            pub struct A(pub u16);
        },
    };

    assert_eq!(diff.to_string(), "≠ A::0 (struct field)\n");
}

#[test]
fn public_named_field_removal_is_modification() {
    let diff = get_diff! {
        {
            pub struct A {
                pub a: u8,
            }
        },
        {
            pub struct A {}
        },
    };

    assert_eq!(diff.to_string(), "- A::a (struct field)\n");
}

#[test]
fn public_unnamed_field_removal_is_modification() {
    let diff = get_diff! {
        {
            pub struct A(pub u8);
        },
        {
            pub struct A();
        },
    };

    assert_eq!(diff.to_string(), "- A::0 (struct field)\n");
}

#[test]
fn generic_change_is_modification() {
    let diff = get_diff! {
        {
            pub struct E;
        },
        {
            pub struct E<T> { f: T }
        },
    };

    assert_eq!(diff.to_string(), "≠ E (struct)\n");
}

#[test]
fn whole_struct_removal() {
    let diff = get_diff! {
        {
            pub struct A {
                pub a: u8,
            }
        },
        {

        },
    };

    assert_eq!(diff.to_string(), "- A (struct)\n");
}
