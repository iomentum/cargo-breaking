use rustc_span::def_id::DefId;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ModMetadata(pub(crate) DefId);

impl ModMetadata {
    pub(crate) fn new(def_id: DefId) -> ModMetadata {
        ModMetadata(def_id)
    }
}
