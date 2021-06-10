#[test]
fn new_public_method_is_addition() {
    let comparator =
        cargo_breaking::compare("pub struct A;", "pub struct A; impl A { pub fn a() {} }").unwrap();
    let diff = comparator.run();

    assert_eq!(diff.to_string(), "+ A::a\n");
}

#[test]
fn new_private_method_is_not_reported() {
    let comparator =
        cargo_breaking::compare("pub struct A;", "pub struct A; impl A { fn a() {} }").unwrap();
    let diff = comparator.run();

    assert!(diff.is_empty());
}

#[test]
fn method_removal_is_removal() {
    let comparator = cargo_breaking::compare(
        "pub struct A; impl A { pub fn a() {} }",
        "pub struct A; impl A {}",
    )
    .unwrap();
    let diff = comparator.run();

    assert_eq!(diff.to_string(), "- A::a\n");
}

#[test]
fn signature_change_is_modification() {
    let comparator = cargo_breaking::compare(
        "pub struct A; impl A { pub fn f(i: u8) {} }",
        "pub struct A; impl A { pub fn f(i: u16) {} }",
    )
    .unwrap();
    let diff = comparator.run();

    assert_eq!(diff.to_string(), "≠ A::f\n");
}
