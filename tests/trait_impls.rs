use cargo_breaking::compatibility_diagnosis;

#[test]
fn addition_simple() {
    let diff = compatibility_diagnosis! {
        {
            pub struct T;
        },
        {
            pub struct T;

            impl A for T {}
        },
    };

    assert_eq!(diff.to_string(), "+ T: A\n");
}

#[test]
fn modification_simple() {
    let diff = compatibility_diagnosis! {
        {
            pub struct T;

            impl<A> A for T<A> {}
        },
        {
            pub struct T;

            impl<B> A for T<B> {}
        }
    };

    assert_eq!(diff.to_string(), "≠ T: A\n");
}

#[test]
fn provided_method_implementation_is_not_reported() {
    let diff = compatibility_diagnosis! {
        {
            pub struct S;

            impl T for S {}
        },
        {
            pub struct S;

            impl T for S {
                fn f() {}
            }
        },
    };

    assert!(diff.is_empty());
}

#[test]
fn constant_modification_is_modification() {
    let diff = compatibility_diagnosis! {
        {
            pub struct S;

            impl T for S {
                const C: usize = 0;
            }
        },
        {
            pub struct S;

            impl T for S {
                const C: usize = 255;
            }
        },
    };

    assert_eq!(diff.to_string(), "≠ S: T\n");
}

#[test]
fn type_modification_is_modification() {
    let diff = compatibility_diagnosis! {
        {
            pub struct S;

            impl T for S {
                type T = u8;
            }
        },
        {
            pub struct S;

            impl T for S {
                type T = u16;
            }
        },
    };

    assert_eq!(diff.to_string(), "≠ S: T\n");
}

#[test]
fn impl_trait_order_is_not_tracked() {
    let diff = compatibility_diagnosis! {
        {
            pub struct S;

            impl T1 for S {}
            impl T2 for S {}
        },
        {
            pub struct S;

            impl T2 for S {}
            impl T1 for S {}
        },
    };

    assert!(diff.is_empty());
}

#[test]
fn not_reported_when_type_is_not_public() {
    let diff = compatibility_diagnosis! {
        {
            struct S;

            impl T for S {}
        },
        {
            struct S;
        }
    };

    assert!(diff.is_empty());
}
