use cargo_breaking::compare;

#[test]
fn addition_simple() {
    let comparator = compare("", "pub trait A {}").unwrap();
    let diff = comparator.run();

    assert_eq!(diff.to_string(), "+ A\n");
}

#[test]
fn trait_item_addition() {
    let comparator = compare("pub trait A {}", "pub trait A { type B; }").unwrap();
    let diff = comparator.run();

    assert_eq!(diff.to_string(), "+ A::B\n");
}

#[test]
fn trait_item_modification() {
    let comparator = compare(
        "pub trait A { type B = u8; }",
        "pub trait A { type B = u16; }",
    )
    .unwrap();
    let diff = comparator.run();

    assert_eq!(diff.to_string(), "≠ A::B\n");
}

#[test]
fn trait_item_removal() {
    let comparator = compare("pub trait A { type B; }", "pub trait A {}").unwrap();
    let diff = comparator.run();

    assert_eq!(diff.to_string(), "- A::B\n");
}

#[test]
fn trait_item_kind_modification() {
    let comparator = compare("pub trait A { type B; }", "pub trait A { const B: usize; }").unwrap();
    let diff = comparator.run();

    assert_eq!(diff.to_string(), "- A::B\n+ A::B\n");
}

#[test]
fn in_private_module() {
    let comparator = compare("", "mod a { pub trait A {} }").unwrap();
    let diff = comparator.run();

    assert!(diff.is_empty());
}

#[test]
fn in_public_module() {
    let comparator = compare("pub mod a {}", "pub mod a { pub trait A {} }").unwrap();
    let diff = comparator.run();

    assert_eq!(diff.to_string(), "+ a::A\n");
}
