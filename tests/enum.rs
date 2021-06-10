#[test]
fn not_reported_when_private() {
    let comparator = cargo_breaking::compare("", "enum A {}").unwrap();
    let diff = comparator.run();

    assert!(diff.is_empty());
}

#[test]
fn new_enum() {
    let comparator = cargo_breaking::compare("", "pub enum A {}").unwrap();
    let diff = comparator.run();

    assert_eq!(diff.to_string(), "+ A\n");
}

#[test]
fn new_named_variant_field_is_modification() {
    let comparator =
        cargo_breaking::compare("pub enum A { B {} }", "pub enum A { B { pub c: u8 } }").unwrap();
    let diff = comparator.run();

    assert_eq!(diff.to_string(), "≠ A\n");
}

#[test]
fn new_unnamed_variant_field_is_modification() {
    let comparator =
        cargo_breaking::compare("pub enum A { B() }", "pub enum A { B(pub u8) }").unwrap();
    let diff = comparator.run();

    assert_eq!(diff.to_string(), "≠ A\n");
}

#[test]
fn named_field_modification() {
    let comparator =
        cargo_breaking::compare("pub enum A { B(pub u8) }", "pub enum A { B(pub u16) }").unwrap();
    let diff = comparator.run();

    assert_eq!(diff.to_string(), "≠ A\n");
}
