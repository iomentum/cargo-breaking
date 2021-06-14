use std::collections::HashMap;

use syn::{
    punctuated::Punctuated,
    token::Comma,
    visit::{self, Visit},
    Field, Fields, FieldsNamed, FieldsUnnamed, Generics, Ident, ItemEnum, ItemMod, ItemStruct,
    Variant, Visibility,
};

use tap::Conv;

#[cfg(test)]
use syn::parse::{Parse, ParseStream, Result as ParseResult};

use super::{traits::TraitImpl, ItemKind, ItemPath};

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct TypeVisitor {
    types: HashMap<ItemPath, ItemKind>,
    path: Vec<Ident>,
}

impl TypeVisitor {
    pub(crate) fn new() -> TypeVisitor {
        TypeVisitor::default()
    }

    pub(crate) fn types(self) -> HashMap<ItemPath, ItemKind> {
        self.types
    }

    fn add_path_segment(&mut self, segment: Ident) {
        self.path.push(segment);
    }

    fn remove_path_segment(&mut self) {
        self.path.pop().unwrap();
    }

    fn add_type(&mut self, path: ItemPath, kind: ItemKind) {
        let tmp = self.types.insert(path, kind);
        assert!(tmp.is_none(), "Duplicate item definition");
    }
}

impl<'ast> Visit<'ast> for TypeVisitor {
    fn visit_item_mod(&mut self, mod_: &'ast ItemMod) {
        if matches!(mod_.vis, Visibility::Public(_)) {
            self.add_path_segment(mod_.ident.clone());
            visit::visit_item_mod(self, mod_);
            self.remove_path_segment();
        }
    }

    fn visit_item_struct(&mut self, i: &'ast ItemStruct) {
        if !matches!(i.vis, Visibility::Public(_)) {
            return;
        }

        let k = ItemPath::new(self.path.clone(), i.ident.clone());
        let v = StructMetadata::new(i.generics.clone(), i.fields.clone())
            .conv::<TypeMetadata>()
            .into();

        self.add_type(k, v);
    }

    fn visit_item_enum(&mut self, i: &'ast ItemEnum) {
        if !matches!(i.vis, Visibility::Public(_)) {
            return;
        }

        let k = ItemPath::new(self.path.clone(), i.ident.clone());
        let v = EnumMetadata::new(i.generics.clone(), i.variants.clone())
            .conv::<TypeMetadata>()
            .into();

        self.add_type(k, v);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TypeMetadata {
    inner: InnerTypeMetadata,
    traits: Vec<TraitImpl>,
}

impl TypeMetadata {
    fn new(inner: InnerTypeMetadata) -> TypeMetadata {
        TypeMetadata {
            inner,
            traits: Vec::new(),
        }
    }
}

impl From<StructMetadata> for TypeMetadata {
    fn from(s: StructMetadata) -> TypeMetadata {
        TypeMetadata::new(s.into())
    }
}

impl From<EnumMetadata> for TypeMetadata {
    fn from(e: EnumMetadata) -> Self {
        TypeMetadata::new(e.into())
    }
}

#[cfg(test)]
impl Parse for TypeMetadata {
    fn parse(input: ParseStream) -> ParseResult<TypeMetadata> {
        Ok(TypeMetadata::new(input.parse()?))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum InnerTypeMetadata {
    Struct(StructMetadata),
    Enum(EnumMetadata),
}

impl From<StructMetadata> for InnerTypeMetadata {
    fn from(v: StructMetadata) -> InnerTypeMetadata {
        InnerTypeMetadata::Struct(v)
    }
}

impl From<EnumMetadata> for InnerTypeMetadata {
    fn from(v: EnumMetadata) -> InnerTypeMetadata {
        InnerTypeMetadata::Enum(v)
    }
}

#[cfg(test)]
impl Parse for InnerTypeMetadata {
    fn parse(input: ParseStream) -> ParseResult<InnerTypeMetadata> {
        input
            .parse::<StructMetadata>()
            .map(Into::into)
            .or_else(|mut e| {
                input.parse::<EnumMetadata>().map(Into::into).map_err(|e_| {
                    e.combine(e_);
                    e
                })
            })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct StructMetadata {
    generics: Generics,
    fields: Fields,
}

impl StructMetadata {
    fn new(generics: Generics, fields: Fields) -> StructMetadata {
        let fields = fields.remove_private_fields();
        StructMetadata { generics, fields }
    }
}

#[cfg(test)]
impl Parse for StructMetadata {
    fn parse(input: ParseStream) -> ParseResult<StructMetadata> {
        let ItemStruct {
            generics, fields, ..
        } = input.parse()?;

        Ok(StructMetadata::new(generics, fields))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct EnumMetadata {
    generics: Generics,
    variants: Vec<Variant>,
}

impl EnumMetadata {
    fn new(generics: Generics, variants: Punctuated<Variant, Comma>) -> EnumMetadata {
        let variants = variants
            .into_iter()
            .map(Variant::remove_private_fields)
            .collect();

        EnumMetadata { generics, variants }
    }
}

#[cfg(test)]
impl Parse for EnumMetadata {
    fn parse(input: ParseStream) -> ParseResult<EnumMetadata> {
        let ItemEnum {
            generics, variants, ..
        } = input.parse()?;
        let variants = variants.into_iter().collect();
        Ok(EnumMetadata { generics, variants })
    }
}

trait ContainsPrivateFields {
    fn remove_private_fields(self) -> Self;
}

impl ContainsPrivateFields for Variant {
    fn remove_private_fields(self) -> Self {
        let Variant {
            attrs,
            ident,
            mut fields,
            discriminant,
        } = self;
        fields = fields.remove_private_fields();

        Variant {
            attrs,
            ident,
            fields,
            discriminant,
        }
    }
}

impl ContainsPrivateFields for Fields {
    fn remove_private_fields(self) -> Self {
        match self {
            Fields::Named(named) => Fields::Named(named.remove_private_fields()),
            Fields::Unnamed(unnamed) => Fields::Unnamed(unnamed.remove_private_fields()),
            Fields::Unit => Fields::Unit,
        }
    }
}

impl ContainsPrivateFields for FieldsNamed {
    fn remove_private_fields(self) -> Self {
        let FieldsNamed {
            brace_token,
            mut named,
        } = self;
        named = named.remove_private_fields();

        FieldsNamed { brace_token, named }
    }
}

impl ContainsPrivateFields for FieldsUnnamed {
    fn remove_private_fields(self) -> Self {
        let FieldsUnnamed {
            paren_token,
            mut unnamed,
        } = self;
        unnamed = unnamed.remove_private_fields();

        FieldsUnnamed {
            paren_token,
            unnamed,
        }
    }
}

impl<U: Default> ContainsPrivateFields for Punctuated<Field, U> {
    fn remove_private_fields(self) -> Self {
        self.into_iter()
            .filter(|field| matches!(field.vis, Visibility::Public(_)))
            .collect()
    }
}
