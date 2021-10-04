use cargo_breaking::compatibility_diagnosis;

#[test]
fn public_addition_is_addition() {
    let diff = compatibility_diagnosis! {
        {
            // Empty file
        },
        {
            pub mod utils {}
        },
    };

    assert_eq!(diff.to_string(), "+ utils\n");
}

#[test]
fn private_addition_not_reported() {
    let diff = compatibility_diagnosis! {
        {
            // Empty file
        },
        {
            mod utils {}
        }
    };

    assert!(diff.is_empty());
}

#[test]
fn removal_is_deletion() {
    let diff = compatibility_diagnosis! {
        {
            pub mod utils {}
        },
        {
            // Empty file
        }
    };

    assert_eq!(diff.to_string(), "- utils\n");
}

#[test]
fn pub_to_pub_crate_is_deletion() {
    let diff = compatibility_diagnosis! {
        {
            pub mod foo {}
        },
        {
            pub(crate) mod foo {}
        },
    };

    assert_eq!(diff.to_string(), "- foo\n");
}
