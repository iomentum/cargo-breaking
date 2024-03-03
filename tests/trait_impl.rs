use cargo_breaking::tests::get_diff;

#[test]
fn addition_simple() {
    let diff = get_diff! {
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

    assert_eq!(diff.to_string(), "+ T::[impl A] (impl)\n");
}

#[test]
fn modification_simple() {
    let diff = get_diff! {
        {
            pub trait A {}

            pub struct T<U> {
                u: U,
            }

            impl<X> A for T<X> {}
        },
        {
            pub trait A {}

            pub struct T<U> {
                u: U,
            }

            impl<Y> A for T<Y> {}
        }
    };

    assert_eq!(diff.to_string(), "≠ T::[impl A] (impl)\n");
}

#[test]
fn provided_method_implementation_is_not_reported() {
    let diff = get_diff! {
        {
            pub trait T {}

            pub struct S;

            impl T for S {}
        },
        {
            pub trait T {
                fn f();
            }

            pub struct S;

            impl T for S {
                fn f() {}
            }
        },
    };

    assert_eq!(diff.to_string(), "+ T::f (method)\n");
}

#[test]
fn constant_modification_is_modification() {
    let diff = get_diff! {
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

    assert_eq!(diff.to_string(), "≠ S::[impl T]::C (associated constant)\n");
}

#[test]
fn type_modification_is_modification() {
    let diff = get_diff! {
        {
            pub trait T {
                type C;
            }

            pub struct S;

            impl T for S {
                type C = u8;
            }
        },
        {
            pub trait T {
                type C;
            }

            pub struct S;

            impl T for S {
                type C = u16;
            }
        },
    };

    assert_eq!(diff.to_string(), "≠ S::[impl T]::C (associated type)\n");
}

#[test]
fn impl_trait_order_is_not_tracked() {
    let diff = get_diff! {
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
    let diff = get_diff! {
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
