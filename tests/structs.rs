#[test]
fn private_is_not_reported() {
    let comparator = cargo_breaking::compare("", "struct A;").unwrap();
    let diff = comparator.run();

    assert!(diff.to_string().is_empty());
}

#[test]
fn addition() {
    let comparator = cargo_breaking::compare("", "pub struct A;").unwrap();
    let diff = comparator.run();

    assert_eq!(diff.to_string(), "+ A\n");
}

#[test]
fn removal() {
    let comparator = cargo_breaking::compare("pub struct B;", "").unwrap();
    let diff = comparator.run();

    assert_eq!(diff.to_string(), "- B\n");
}

#[test]
fn new_public_field_tupled() {
    let comparator = cargo_breaking::compare("pub struct C;", "pub struct C(pub u8);").unwrap();
    let diff = comparator.run();

    assert_eq!(diff.to_string(), "+ C::0\n");
}

#[test]
fn new_private_field_tupled() {
    let comparator = cargo_breaking::compare("pub struct C();", "pub struct C(usize);").unwrap();
    let diff = comparator.run();

    assert!(diff.to_string().is_empty());
}

#[test]
fn new_public_field_named() {
    let comparator =
        cargo_breaking::compare("pub struct D {}", "pub struct D { pub a: u8 }").unwrap();
    let diff = comparator.run();

    assert_eq!(diff.to_string(), "+ D::a\n");
}

#[test]
fn new_private_field_named() {
    let comparator =
        cargo_breaking::compare("pub struct D { a: b }", "pub struct D { a: b, c: d }").unwrap();
    let diff = comparator.run();

    assert!(diff.to_string().is_empty());
}

#[test]
fn public_named_field_modification() {
    let comparator =
        cargo_breaking::compare("pub struct A { pub a: u8 }", "pub struct A { pub a: u16 }")
            .unwrap();
    let diff = comparator.run();

    assert_eq!(diff.to_string(), "≠ A::a\n");
}

#[test]
fn public_unnamed_field_modification() {
    let comparator =
        cargo_breaking::compare("pub struct A(pub u8);", "pub struct A(pub u16);").unwrap();
    let diff = comparator.run();

    assert_eq!(diff.to_string(), "≠ A::0\n");
}

#[test]
fn public_named_field_removal() {
    let comparator =
        cargo_breaking::compare("pub struct A { pub a: u8 }", "pub struct A {}").unwrap();
    let diff = comparator.run();

    assert_eq!(diff.to_string(), "- A::a\n");
}

#[test]
fn public_unnamed_field_removal() {
    let comparator = cargo_breaking::compare("pub struct A(pub u8);", "pub struct A();").unwrap();
    let diff = comparator.run();

    assert_eq!(diff.to_string(), "- A::0\n");
}

#[test]
fn generic_change() {
    let comparator = cargo_breaking::compare("pub struct E;", "pub struct E<T>;").unwrap();
    let diff = comparator.run();

    assert_eq!(diff.to_string(), "≠ E\n");
}
