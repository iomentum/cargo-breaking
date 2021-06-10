#[test]
fn private_is_not_reported() {
    let comparator = cargo_breaking::compare("", "fn fact(n: u32) -> u32 {}").unwrap();
    let rslt = comparator.run();

    assert!(rslt.is_empty());
}

#[test]
fn addition() {
    let comparator = cargo_breaking::compare("", "pub fn fact(n: u32) -> u32 {}").unwrap();
    let rslt = comparator.run();

    assert_eq!(rslt.to_string(), "+ fact\n");
}

#[test]
fn new_arg() {
    let comparator = cargo_breaking::compare("pub fn fact() {}", "pub fn fact(n: u32) {}").unwrap();
    let rslt = comparator.run();

    assert_eq!(rslt.to_string(), "≠ fact\n");
}

#[test]
fn generic_order() {
    let comparator = cargo_breaking::compare("pub fn f<T, E>() {}", "pub fn f<E, T>() {}").unwrap();
    let rslt = comparator.run();

    assert_eq!(rslt.to_string(), "≠ f\n");
}

#[test]
fn body_change_not_detected() {
    let comparator =
        cargo_breaking::compare("pub fn fact() {}", "pub fn fact() { todo!() }").unwrap();
    let rslt = comparator.run();

    assert!(rslt.is_empty());
}

#[test]
fn empty_struct_kind_change_is_modification() {
    let files = ["pub struct A;", "pub struct A();", "pub struct A {}"];

    for (id_a, file_a) in files.iter().enumerate() {
        for (id_b, file_b) in files.iter().enumerate() {
            let comparator = cargo_breaking::compare(file_a, file_b).unwrap();
            let diff = comparator.run();

            if id_a != id_b {
                assert_eq!(diff.to_string(), "≠ A\n");
            } else {
                assert!(diff.is_empty());
            }
        }
    }
}
