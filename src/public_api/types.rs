use rustdoc_types::{Crate, Enum, Struct, StructType, Typedef, Union};

use crate::diagnosis::DiagnosticGenerator;

use crate::rustdoc::types::{Generics, RustdocToCb, Type};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TypeMetadata {
    generics: Generics,
    data: InnerTypeMetadata,
}

impl TypeMetadata {
    fn new(generics: Generics, data: InnerTypeMetadata) -> TypeMetadata {
        TypeMetadata { generics, data }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum InnerTypeMetadata {
    Struct {
        struct_type: StructType,
        fields_stripped: bool,
    },
    Union {
        fields_stripped: bool,
    },
    Enum {
        variants_stripped: bool,
    },
    Typedef {
        type_: Box<Type>,
    },
}

impl PartialEq for InnerTypeMetadata {
    fn eq(&self, other: &Self) -> bool {
        // special case: when a type containing only public items/fields gets added a private field
        // for structs and unions this breaks literals
        // the change is not breaking, since it only *allows* new usages
        if !self.is_stripped() && other.is_stripped() {
            return false;
        }

        use InnerTypeMetadata::*;
        match (self, other) {
            (
                Struct { struct_type, .. },
                Struct {
                    struct_type: struct_type_other,
                    ..
                },
            ) => struct_type == struct_type_other,
            (Typedef { type_ }, Typedef { type_: type_other }) => type_ == type_other,
            _ => true,
        }
    }
}

impl InnerTypeMetadata {
    fn is_stripped(&self) -> bool {
        match self {
            InnerTypeMetadata::Struct {
                fields_stripped, ..
            } => *fields_stripped,
            InnerTypeMetadata::Union {
                fields_stripped, ..
            } => *fields_stripped,
            InnerTypeMetadata::Enum {
                variants_stripped, ..
            } => *variants_stripped,
            InnerTypeMetadata::Typedef { .. } => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TypeFieldMetadata {
    name: String,
    ty: Type,
}

impl TypeFieldMetadata {
    pub fn new(name: String, ty: Type) -> TypeFieldMetadata {
        TypeFieldMetadata { name, ty }
    }
}

impl DiagnosticGenerator for TypeFieldMetadata {}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct EnumVariantMetadata {
    name: String,
}

impl EnumVariantMetadata {
    pub fn new(name: String) -> EnumVariantMetadata {
        EnumVariantMetadata { name }
    }
}

impl DiagnosticGenerator for EnumVariantMetadata {}

impl RustdocToCb<TypeMetadata> for Struct {
    fn to_cb(&self, data: &Crate) -> TypeMetadata {
        TypeMetadata::new(
            self.generics.to_cb(data),
            InnerTypeMetadata::Struct {
                struct_type: self.struct_type.clone(),
                fields_stripped: self.fields_stripped,
            },
        )
    }
}

impl RustdocToCb<TypeMetadata> for Union {
    fn to_cb(&self, data: &Crate) -> TypeMetadata {
        TypeMetadata::new(
            self.generics.to_cb(data),
            InnerTypeMetadata::Union {
                fields_stripped: self.fields_stripped,
            },
        )
    }
}

impl RustdocToCb<TypeMetadata> for Enum {
    fn to_cb(&self, data: &Crate) -> TypeMetadata {
        TypeMetadata::new(
            self.generics.to_cb(data),
            InnerTypeMetadata::Enum {
                variants_stripped: self.variants_stripped,
            },
        )
    }
}

impl RustdocToCb<TypeMetadata> for Typedef {
    fn to_cb(&self, data: &Crate) -> TypeMetadata {
        TypeMetadata::new(
            self.generics.to_cb(data),
            InnerTypeMetadata::Typedef {
                type_: Box::new(self.type_.to_cb(data)),
            },
        )
    }
}

impl DiagnosticGenerator for TypeMetadata {}
