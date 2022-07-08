use cargo_breaking::tests::get_diff;

#[test]
fn new_public_method_is_addition() {
    let diff = get_diff! {
        {
            pub struct A;
        },
        {
            pub struct A;

            impl A {
                pub fn a() {}
            }
        },
    };

    assert_eq!(diff.to_string(), "+ A::a (method)\n");
}

#[test]
fn new_private_method_is_not_reported() {
    let diff = get_diff! {
        {
            pub struct A;
        },
        {
            pub struct A;

            impl A {
                fn a() {}
            }
        },
    };

    assert!(diff.is_empty());
}

#[test]
fn method_removal_is_removal() {
    let diff = get_diff! {
        {
            pub struct A;

            impl A {
                pub fn a() {}
            }
        },
        {
            pub struct A;

            impl A {}
        }
    };

    assert_eq!(diff.to_string(), "- A::a (method)\n");
}

#[test]
fn signature_change_is_modification() {
    let diff = get_diff! {
        {
            pub struct A;

            impl A {
                pub fn f(i: u8) {}
            }
        },
        {
            pub struct A;

            impl A {
                pub fn f(i: u16) {}
            }
        },
    };

    assert_eq!(diff.to_string(), "≠ A::f (method)\n");
}

#[test]
fn generic_param_change_is_not_modification() {
    let diff = get_diff! {
        {
            pub struct A;
            impl<T> A {
                pub fn f() {}
            }
        },
        {
            pub struct A;
            impl<U> A {
                pub fn f() {}
            }
        }
    };

    assert!(diff.is_empty());
}

#[test]
fn generic_arg_change_is_modification() {
    let diff = get_diff! {
        {
            pub struct A<T> {
                t: T,
            }

            impl A<u8> {
                pub fn f() {}
            }
        },
        {
            pub struct A<T> {
                t: T,
            }

            impl A<u16> {
                pub fn f() {}
            }
        },
    };

    assert_eq!(
        diff.to_string(),
        "- A::<u8>::f (method)\n+ A::<u16>::f (method)\n"
    );
}

#[test]
fn not_reported_when_type_is_not_public() {
    let diff = get_diff! {
        {
            struct A;

            impl A {}
        },
        {
            struct A;

            impl A {
                fn f() {}
            }
        },
    };

    assert!(diff.is_empty());
}

#[test]
fn is_reported_in_type_definition_path_1() {
    let diff = get_diff! {
        {
            pub mod foo {
                pub struct Bar;
            }

            impl foo::Bar {
                pub fn f() {}
            }
        },
        {
            pub mod foo {
                pub struct Bar;
            }
        },
    };

    assert_eq!(diff.to_string(), "- foo::Bar::f (method)\n");
}

#[test]
fn is_reported_in_type_definition_path_2() {
    let diff = get_diff! {
        {
            pub mod foo {
                pub struct Bar;
            }

            pub mod baz {
                impl crate::foo::Bar {
                    pub fn f() {}
                }
            }
        },
        {
            pub mod foo {
                pub struct Bar;
            }
        }
    };

    assert_eq!(diff.to_string(), "- baz (module)\n- foo::Bar::f (method)\n");
}
