#[test]
fn addition_simple() {
    let comparator =
        cargo_breaking::compare("pub struct T;", "pub struct T; impl A for T {}").unwrap();
    let diff = comparator.run();

    assert_eq!(diff.to_string(), "+ T: A\n");
}

#[test]
fn modification_simple() {
    let comparator = cargo_breaking::compare(
        "pub struct T; impl<A> A for T<A> {}",
        "pub struct T; impl<B> A for T<B> {}",
    )
    .unwrap();
    let diff = comparator.run();

    assert_eq!(diff.to_string(), "≠ T: A\n");
}

#[test]
fn provided_method_implementation_is_not_reported() {
    let comparator = cargo_breaking::compare(
        "pub struct S; impl T for S {}",
        "pub struct S; impl T for S { fn f() {} }",
    )
    .unwrap();
    let diff = comparator.run();

    assert!(diff.is_empty());
}

#[test]
fn constant_modification_is_modification() {
    let comparator = cargo_breaking::compare(
        "pub struct S; impl T for S { const C: usize = 0; }",
        "pub struct S; impl T for S { const C: usize = 255; }",
    )
    .unwrap();
    let diff = comparator.run();

    assert_eq!(diff.to_string(), "≠ S: T\n");
}

#[test]
fn type_modification_is_modification() {
    let comparator = cargo_breaking::compare(
        "pub struct S; impl T for S { type T = u8; }",
        "pub struct S; impl T for S { type T = u16; }",
    )
    .unwrap();
    let diff = comparator.run();

    assert_eq!(diff.to_string(), "≠ S: T\n");
}

#[test]
fn impl_trait_order_is_not_tracked() {
    let comparator = cargo_breaking::compare(
        "pub struct S; impl T1 for S {} impl T2 for S {}",
        "pub struct S; impl T2 for S {} impl T1 for S {}",
    )
    .unwrap();
    let diff = comparator.run();

    assert!(diff.is_empty());
}
