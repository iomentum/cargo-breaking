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

#[test]
fn generic_param_change_is_modification() {
    let comparator = cargo_breaking::compare(
        "pub struct A; impl<T> A { pub fn f() {} }",
        "pub struct A; impl<U> A { pub fn f() {} }",
    )
    .unwrap();
    let diff = comparator.run();

    assert_eq!(diff.to_string(), "≠ A::f\n");
}

#[test]
fn generic_arg_change_is_modification() {
    let comparator = cargo_breaking::compare(
        "pub struct A; impl A<T> { pub fn f() {} }",
        "pub struct A; impl A<U> { pub fn f() {} }",
    )
    .unwrap();
    let diff = comparator.run();

    assert_eq!(diff.to_string(), "≠ A::f\n");
}

#[test]
fn not_reported_when_type_is_not_public() {
    let comparator =
        cargo_breaking::compare("struct A; impl A {}", "struct A; impl A { fn f() {} }").unwrap();
    let diff = comparator.run();

    assert!(diff.is_empty());
}

#[test]
fn is_reported_in_type_definition_path_1() {
    let comparator = cargo_breaking::compare(
        "pub mod foo { pub struct Bar; } impl foo::Bar { pub fn f() {} }",
        "pub mod foo { pub struct Bar; }",
    )
    .unwrap();
    let diff = comparator.run();

    assert_eq!(diff.to_string(), "- foo::Bar::f\n");
}

#[test]
fn is_reported_in_type_definition_path_2() {
    let comparator = cargo_breaking::compare(
        "pub mod foo { pub struct Bar; } pub mod baz { impl crate::foo::Bar { pub fn f() {} } }",
        "pub mod foo { pub struct Bar; }",
    )
    .unwrap();

    let diff = comparator.run();

    assert_eq!(diff.to_string(), "- foo::Bar::f\n");
}
