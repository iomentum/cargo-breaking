use cargo_breaking::compatibility_diagnosis;

#[test]
fn addition_simple() {
    let diff = compatibility_diagnosis! {
        {
            pub trait A {}
            pub struct T;
        },
        {
            pub trait A {}
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
            pub trait T {}
            pub struct S<T>(T);

            impl<A> T for S<A> {}
        },
        {
            pub trait T {}
            pub struct S<T>(T);

            impl<B> T for S<B> {}
        }
    };

    assert!(diff.is_empty());
}

#[test]
fn provided_method_implementation_is_not_reported() {
    let diff = compatibility_diagnosis! {
        {
            pub trait T {
                fn f() {}
            }
            pub struct S;

            impl T for S {}
        },
        {
            pub trait T {
                fn f() {}
            }
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
            pub trait T {
                const C: usize;
            }
            pub struct S;

            impl T for S {
                const C: usize = 0;
            }
        },
        {
            pub trait T {
                const C: usize;
            }
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
            pub trait T {
                type T;
            }
            pub struct S;

            impl T for S {
                type T = u8;
            }
        },
        {
            pub trait T {
                type T;
            }
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
            pub trait T1 {}
            pub trait T2 {}
            pub struct S;

            impl T1 for S {}
            impl T2 for S {}
        },
        {
            pub trait T1 {}
            pub trait T2 {}
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
            pub trait T {}
            struct S;

            impl T for S {}
        },
        {
            pub trait T {}
            struct S;
        }
    };

    assert!(diff.is_empty());
}
