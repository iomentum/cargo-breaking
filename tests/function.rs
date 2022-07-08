use cargo_breaking::tests::get_diff;

#[test]
fn private_is_not_reported() {
    let diff = get_diff! {
        {},
        {
            fn fact(n: u32) -> u32 { todo!() }
        },
    };

    assert!(diff.is_empty());
}

#[test]
fn addition() {
    let diff = get_diff! {
        {},
        {
            pub fn fact(n: u32) -> u32 { todo!() }
        },
    };

    assert_eq!(diff.to_string(), "+ fact (function)\n");
}

#[test]
fn new_arg() {
    let diff = get_diff! {
        {
            pub fn fact() {}
        },
        {
            pub fn fact(n: u32) {}
        }
    };

    assert_eq!(diff.to_string(), "≠ fact (function)\n");
}

#[test]
fn generic_order() {
    let diff = get_diff! {
        {
            pub fn f<T, E>() {}
        },
        {
            pub fn f<E, T>() {}
        },
    };

    assert_eq!(diff.to_string(), "≠ f (function)\n");
}

#[test]
fn body_change_not_detected() {
    let diff = get_diff! {
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
fn fn_arg_last_character_not_removed() {
    let diff = get_diff! {
        {
            pub fn a(a: u8, b: u8, c: u8) {}
        },
        {
            pub fn a(a: u8, b: u8, c: u16) {}
        },
    };

    assert_eq!(diff.to_string(), "≠ a (function)\n");
}

#[test]
fn is_reported_lexicographically() {
    let diff = get_diff! {
        {},
        {
            pub fn a() {}
            pub fn z() {}
        }
    };
    assert_eq!(diff.to_string(), "+ a (function)\n+ z (function)\n");

    let diff = get_diff! {
        {},
        {
            pub fn z() {}
            pub fn a() {}
        }
    };
    assert_eq!(diff.to_string(), "+ a (function)\n+ z (function)\n");
}

#[test]
fn custom_type_equality() {
    let diff = get_diff! {
        {
            pub struct A { pub a: u8 }

            pub fn test(a: A) {}
        },
        {
            pub struct A { pub a: u16 }

            pub fn test(a: A) {}
        },
    };

    assert_eq!(diff.to_string(), "≠ A::a (struct field)\n");
}
