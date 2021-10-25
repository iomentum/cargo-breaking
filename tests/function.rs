use cargo_breaking::compatibility_diagnosis;

#[test]
fn private_is_not_reported() {
    let diff = compatibility_diagnosis! {
        {},
        {
            fn fact(n: u32) -> u32 { todo!() }
        },
    };

    assert!(diff.is_empty());
}

#[test]
fn addition() {
    let diff = compatibility_diagnosis! {
        {},
        {
            pub fn fact(n: u32) -> u32 { todo!() }
        },
    };

    assert_eq!(diff.to_string(), "+ fact\n");
}

#[test]
fn new_arg() {
    let diff = compatibility_diagnosis! {
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
    let diff = compatibility_diagnosis! {
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
    let diff = compatibility_diagnosis! {
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
    let diff = compatibility_diagnosis! {
        {
            type t = ();
            pub fn a(a: t, b: t, c: t,) {}
        },
        {
            type t = ();
            pub fn a(a: t, b: t, c: t) {}
        },
    };

    assert!(diff.is_empty());
}

#[test]
fn fn_arg_last_character_not_removed() {
    let diff = compatibility_diagnosis! {
        {
            type t = ();
            pub fn a(a: t, b: t, c: t) {}
        },
        {
            type t = ();
            type u = u8;
            pub fn a(a: t, b: t, c: u) {}
        },
    };

    assert_eq!(diff.to_string(), "≠ a\n");
}

#[test]
fn user_defined_type_change_is_not_reported() {
    let diff = compatibility_diagnosis! {
        {
            pub struct S;
            pub fn f(_: S) {}
        },
        {
            pub struct S;
            pub fn f(_: S) {}
        },
    };

    assert!(diff.is_empty());
}

#[test]
fn empty_struct_kind_change_is_modification() {
    let files = ["pub struct A;", "pub struct A();", "pub struct A {}"];

    for (id_a, file_a) in files.iter().enumerate() {
        for (id_b, file_b) in files.iter().enumerate() {
            let diff = cargo_breaking::get_diff_from_sources(file_a, file_b).unwrap();

            if id_a != id_b {
                assert_eq!(diff.to_string(), "≠ A\n");
            } else {
                assert!(diff.is_empty());
            }
        }
    }
}

#[test]
fn is_reported_lexicographically() {
    let diff = compatibility_diagnosis! {
        {},
        {
            pub fn a() {}
            pub fn z() {}
        }
    };
    assert_eq!(diff.to_string(), "+ a\n+ z\n");

    let diff = compatibility_diagnosis! {
        {},
        {
            pub fn z() {}
            pub fn a() {}
        }
    };
    assert_eq!(diff.to_string(), "+ a\n+ z\n");
}
