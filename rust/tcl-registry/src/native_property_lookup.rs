// SPDX-License-Identifier: AGPL-3.0-or-later
//! C9.1 property table ownership and Foundation/instance invalidation recipes.
use tcl_dialect::TclVersion;
use tcl_syntax::native_string::NativeStringProtocol;
use tcl_syntax::value::ValueError;

/// Property-cache epoch affected by an accepted native C9.1 mutation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativePropertyInvalidation {
    /// Keep the existing property cache epoch and retained headers.
    None,
    /// Advance the interpreter's OO Foundation epoch.
    Foundation,
    /// Advance the affected instance's property cache epoch.
    Instance,
    /// Advance the class representative object's property cache epoch.
    ClassRepresentative,
}
/// Actual Foundation graph links affected by a class structure mutation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NativePropertyGraphDependents {
    /// Another class retains the affected class in its superclass links.
    pub subclasses: bool,
    /// An object retains the affected class as its class or a mixin.
    pub instances: bool,
    /// Another class retains the affected class in its mixin links.
    pub mixin_dependents: bool,
}
/// Sealed actual C9.1 property lookup, accessor production, and invalidation recipe.
#[derive(Clone, Copy, Debug)]
pub struct NativePropertyLookupProtocol {
    materialization: crate::native_string_materialization::NativeStringMaterialization,
}
impl NativePropertyLookupProtocol {
    /// `Tcl_NewMethod` always changes Foundation; instance method creation only
    /// changes its method-chain epoch, leaving property headers resident.
    #[must_use]
    pub const fn method_created(self, class: bool) -> NativePropertyInvalidation {
        if class {
            NativePropertyInvalidation::Foundation
        } else {
            NativePropertyInvalidation::None
        }
    }
    /// `TclOODefineCmds` BumpGlobalEpoch/BumpInstanceEpoch, using actual graph links.
    #[must_use]
    pub const fn structure_changed(
        self,
        class: bool,
        dependents: NativePropertyGraphDependents,
        representative_mixins: bool,
    ) -> NativePropertyInvalidation {
        if !class {
            NativePropertyInvalidation::Instance
        } else if dependents.subclasses || dependents.instances || dependents.mixin_dependents {
            NativePropertyInvalidation::Foundation
        } else if representative_mixins {
            NativePropertyInvalidation::ClassRepresentative
        } else {
            NativePropertyInvalidation::None
        }
    }
    /// Validate an original counted declaration using native property-name rules.
    ///
    /// # Errors
    /// Returns the native byte diagnostic for a forbidden initial dash, quoting,
    /// namespace separator, or parenthesis in the selected name extent.
    pub fn validate_declaration(self, original: &[u8]) -> Result<(), Vec<u8>> {
        let name = tcl_core_types::c_string_extent(original);
        let mut quoted = Vec::new();
        tcl_syntax::list::append_list_element(&mut quoted, original, false);
        let reason = if name.first() == Some(&b'-') {
            Some(b"must not begin with -".as_slice())
        } else if quoted.len() != original.len() {
            Some(b"must be a simple word".as_slice())
        } else if name.windows(2).any(|pair| pair == b"::") {
            Some(b"must not contain namespace separators".as_slice())
        } else if name.iter().any(|byte| matches!(byte, b'(' | b')')) {
            Some(b"must not contain parentheses".as_slice())
        } else {
            None
        };
        if let Some(reason) = reason {
            let mut message = b"bad property name \"".to_vec();
            message.extend_from_slice(name);
            message.extend_from_slice(b"\": ");
            message.extend_from_slice(reason);
            Err(message)
        } else {
            Ok(())
        }
    }
    /// Retain the actual ordered static table for property definition options.
    #[must_use]
    pub fn definition_options(self) -> crate::native_index_lookup::NativeStaticIndexTable {
        crate::native_index_lookup::NativeStaticIndexTable::supported_backend(&[
            "-get", "-kind", "-set",
        ])
    }
    /// Retain the actual ordered static table for property access kinds.
    #[must_use]
    pub fn definition_kinds(self) -> crate::native_index_lookup::NativeStaticIndexTable {
        crate::native_index_lookup::NativeStaticIndexTable::supported_backend(&[
            "readable",
            "readwrite",
            "writable",
        ])
    }
    /// Select `TclOOLookupObjectVar`'s original operand producer. An absolute
    /// `CString` prefix retains the supplied header; a relative name makes one
    /// fresh header from the actual namespace and the full counted name.
    /// `None` means retain the original header, including its physical primary.
    #[must_use]
    pub fn object_variable_operand(self, original: &[u8], namespace: &[u8]) -> Option<Vec<u8>> {
        if tcl_core_types::c_string_extent(original).starts_with(b"::") {
            return None;
        }
        let mut qualified = namespace.to_vec();
        qualified.extend_from_slice(b"::");
        qualified.extend_from_slice(original);
        Some(qualified)
    }
    /// `Tcl_ObjPrintf` accessors use the original property's C string extent.
    #[must_use]
    pub fn accessor_names(self, original: &[u8]) -> (Vec<u8>, Vec<u8>) {
        let name = tcl_core_types::c_string_extent(original);
        let make = |prefix: &[u8]| {
            let mut bytes = prefix.to_vec();
            bytes.extend_from_slice(name);
            bytes.push(b'>');
            bytes
        };
        (make(b"<ReadProp"), make(b"<WriteProp"))
    }
    /// Return the authenticated formatted-String producer for accessor children.
    #[must_use]
    pub const fn materialization(
        self,
    ) -> crate::native_string_materialization::NativeStringMaterialization {
        self.materialization
    }
    /// The borrowed native List table is `TEMP_TABLE`. No Index cache can be
    /// consumed or installed; each reached query materializes its original name.
    ///
    /// # Errors
    /// Returns the native property lookup diagnostic for an unknown or ambiguous
    /// C-string name against the supplied original ordered table members.
    pub fn lookup(self, original: &[u8], members: &[&[u8]]) -> Result<usize, Vec<u8>> {
        tcl_cmd_core::prefix::OptionTable::abbreviating("property", members)
            .index_of(tcl_core_types::c_string_extent(original))
    }
    /// Return the actual C9.1 string protocol retained by this recipe.
    #[must_use]
    pub const fn strings(self) -> NativeStringProtocol {
        self.materialization.protocol()
    }
    /// Reject a foreign physical Index primary before a `TEMP_TABLE` name getter.
    /// This check grants no cache hit or installation authority.
    ///
    /// # Errors
    /// Returns a typed host refusal when the existing Index origin is not C9.1.
    pub fn validate_index_origin(self, origin: Option<TclVersion>) -> Result<(), ValueError> {
        if origin.is_some_and(|version| version != TclVersion::V9_1) {
            Err(ValueError::CommandProtocolUnavailable(
                "property original Index origin",
            ))
        } else {
            Ok(())
        }
    }
}
impl crate::InvocationDialect {
    /// Select this recipe only from an authenticated actual C9.1 string issuer.
    /// Other releases, Jim, and unavailable issuers return `None`.
    #[must_use]
    pub fn native_property_lookup_protocol(self) -> Option<NativePropertyLookupProtocol> {
        let materialization = self.native_string_materialization(None)?;
        (materialization.protocol() == NativeStringProtocol::C(TclVersion::V9_1))
            .then_some(NativePropertyLookupProtocol { materialization })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn property_temporary_lookup_and_mutation_axes_are_independent() {
        let recipe = crate::InvocationDialect::of_profile(
            tcl_dialect::DialectProfile::find("tcl9.1").unwrap(),
        )
        .native_property_lookup_protocol()
        .unwrap();
        assert_eq!(
            recipe.object_variable_operand(b"x\0tail", b"::oo::Obj1"),
            Some(b"::oo::Obj1::x\0tail".to_vec())
        );
        assert_eq!(
            recipe.object_variable_operand(b"::global\0tail", b"::oo::Obj1"),
            None
        );
        assert_eq!(
            recipe.lookup(b"-y\0ignored", &[b"-yellow", b"-zinc"]),
            Ok(0)
        );
        assert!(recipe.lookup(b"-", &[b"-yellow", b"-zinc"]).is_err());
        assert_eq!(
            recipe.method_created(false),
            NativePropertyInvalidation::None
        );
        assert_eq!(
            recipe.method_created(true),
            NativePropertyInvalidation::Foundation
        );
        assert_eq!(
            recipe.structure_changed(true, NativePropertyGraphDependents::default(), false),
            NativePropertyInvalidation::None
        );
        assert_eq!(
            recipe.structure_changed(true, NativePropertyGraphDependents::default(), true),
            NativePropertyInvalidation::ClassRepresentative
        );
        for (subclasses, instances, mixins) in [
            (true, false, false),
            (false, true, false),
            (false, false, true),
        ] {
            assert_eq!(
                recipe.structure_changed(
                    true,
                    NativePropertyGraphDependents {
                        subclasses,
                        instances,
                        mixin_dependents: mixins,
                    },
                    false,
                ),
                NativePropertyInvalidation::Foundation
            );
        }
        assert_eq!(
            recipe.structure_changed(false, NativePropertyGraphDependents::default(), false),
            NativePropertyInvalidation::Instance
        );
        let dependents = NativePropertyGraphDependents {
            subclasses: true,
            instances: true,
            mixin_dependents: true,
        };
        assert_eq!(
            recipe.structure_changed(true, dependents, true),
            NativePropertyInvalidation::Foundation
        );
        assert_eq!(
            recipe.structure_changed(false, dependents, true),
            NativePropertyInvalidation::Instance
        );
        assert!(
            recipe
                .validate_index_origin(Some(TclVersion::V8_6))
                .is_err()
        );
        assert!(recipe.validate_index_origin(Some(TclVersion::V9_1)).is_ok());
        let rows = include_str!("../tests/data/native_property_owners/native.tsv");
        assert_eq!(rows.lines().count(), 8);
        assert!(
            rows.lines()
                .any(|row| row.starts_with("class-method-error\t33\t1\t1\t"))
        );
    }
}
