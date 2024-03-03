# Comparison

The `Crate` object from Rustdoc is converted to a `PublicApi` object, which is a flat hashmap of all public items in 
the library.

## Translation

The `Crate` object is traversed and all its items associated to the root crate (ID 0) are processed.

### Functions

Free functions and methods yield `Fn` and `Method` items, respectively.

### Types

Structures, unions and enums yield `Type` items, and their fields and impls are also traversed.

#### Fields

Fields yield `Field` items.

#### Impls

Impl items are processed and yield either `Method`, `AssocType` or `AssocConst` items.

If the impl is for a trait, it yields a `TraitImpl` item.

### Trait definitions

Trait definitions yield `TraitDef` items, and their items are processed in the same way as for impl items.

### Modules

Modules yield `Module` items.

Their items are not directly processed since they already appear in the `Crate` index.

### Typedefs

Typedefs yield `Type` items.