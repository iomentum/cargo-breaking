use syn::{ItemFn, Path, Signature};

#[derive(Clone, Debug, PartialEq)]
struct PublicFn {
    path: Path,
    sig: Signature,
}

impl PublicFn {
    // TODO: handle cases when the function is not actually public (eg: return none)
    fn from(s_fn: ItemFn, path: Path) -> PublicFn {
        let sig = s_fn.sig;

        PublicFn { path, sig }
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use syn::{parse_str, ItemFn};

    use super::*;

    fn sample_path() -> Path {
        parse_str("crate::foo").unwrap()
    }

    fn sample_private_fn() -> ItemFn {
        parse_str("fn fact(n: u32) -> u32 {}").unwrap()
    }

    #[test]
    fn fn_() -> Result<(), Box<dyn Error>> {
        let left = PublicFn::from(sample_private_fn(), sample_path());
        let right = PublicFn {
            path: sample_path(),
            sig: parse_str("fn fact(n: u32) -> u32")?,
        };

        assert_eq!(left, right);

        Ok(())
    }
}
