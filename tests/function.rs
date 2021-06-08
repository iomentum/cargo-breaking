use cargo_breaking;

#[test]
fn addition() {
    let comparator = cargo_breaking::compare("", "fn fact(n: u32) -> u32 {}").unwrap();
    let rslt = comparator.run();

    assert_eq!(rslt.to_string(), "+ fact\n");
}

#[test]
fn new_arg() {
    let comparator = cargo_breaking::compare("fn fact() {}", "fn fact(n: u32) {}").unwrap();
    let rslt = comparator.run();

    assert_eq!(rslt.to_string(), "≠ fact\n");
}

#[test]
fn generic_order() {
    let comparator = cargo_breaking::compare("fn f<T, E>() {}", "fn f<E, T>() {}").unwrap();
    let rslt = comparator.run();

    assert_eq!(rslt.to_string(), "≠ f\n");
}

#[test]
fn body_change_not_detected() {
    let comparator = cargo_breaking::compare("fn fact() {}", "fn fact() { todo!() }").unwrap();
    let rslt = comparator.run();

    assert!(rslt.to_string().is_empty());
}
