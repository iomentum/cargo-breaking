use cargo_breaking;

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
fn new_field_tupled() {
    let comparator = cargo_breaking::compare("pub struct C();", "pub struct C(usize);").unwrap();
    let diff = comparator.run();

    assert_eq!(diff.to_string(), "≠ C\n");
}

#[test]
fn new_field_named() {
    let comparator =
        cargo_breaking::compare("pub struct D { a: b }", "pub struct D { a: b, c: d }").unwrap();
    let diff = comparator.run();

    assert_eq!(diff.to_string(), "≠ D\n")
}

#[test]
fn generic_change() {
    let comparator = cargo_breaking::compare("pub struct E;", "pub struct E<T>;").unwrap();
    let diff = comparator.run();

    assert_eq!(diff.to_string(), "≠ E\n");
}
