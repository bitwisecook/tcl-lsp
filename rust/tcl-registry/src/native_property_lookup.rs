// SPDX-License-Identifier: AGPL-3.0-or-later
//! C9 counted property names and independent C9.1 physical property ownership.
use tcl_dialect::TclVersion;
use tcl_syntax::native_string::NativeStringProtocol;
use tcl_syntax::value::ValueError;

/// Independently selected C9 property declaration validity. This pure
/// counted/C-string/list-scan recipe grants no property table, accessor header,
/// cache generation or successful registration.
///
/// # Errors
/// The contained result retains the actual native byte diagnostic for an
/// invalid name. `None` means the requested engine purpose is unmodelled.
#[must_use]
pub fn native_property_declaration_validity(
    dialect: crate::InvocationDialect,
    original: &[u8],
) -> Option<Result<(), Vec<u8>>> {
    (dialect.family() == Some(tcl_dialect::model::Family::Tcl)
        && dialect
            .tcl_version
            .is_some_and(|version| version >= TclVersion::V9_0))
    .then(|| property_declaration_validity(original))
}

fn property_declaration_validity(original: &[u8]) -> Result<(), Vec<u8>> {
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

/// Selected C9 property name, declaration and accessor operand semantics.
/// This pure recipe supplies no property header, cache epoch or physical table.
#[derive(Clone, Copy, Debug)]
pub struct NativePropertyNameProtocol {
    strings: NativeStringProtocol,
}

impl NativePropertyNameProtocol {
    /// Counted native string recipe selected independently of property storage.
    #[must_use]
    pub const fn strings(self) -> NativeStringProtocol {
        self.strings
    }

    /// Select declaration validity without issuing physical lookup authority.
    ///
    /// # Errors
    /// Returns the native byte diagnostic for an invalid original name.
    pub fn validate_declaration(self, original: &[u8]) -> Result<(), Vec<u8>> {
        property_declaration_validity(original)
    }

    /// Retain the actual authored option roster in native table order.
    #[must_use]
    pub fn definition_options(self) -> crate::native_index_lookup::NativeStaticIndexTable {
        property_definition_options()
    }
    /// Retain the actual authored access-kind roster in native table order.
    #[must_use]
    pub fn definition_kinds(self) -> crate::native_index_lookup::NativeStaticIndexTable {
        property_definition_kinds()
    }
    /// Formatted property membership uses the original `CString` name extent.
    #[must_use]
    pub fn dashed_name(self, original: &[u8]) -> Vec<u8> {
        tcl_syntax::naming::NativeNameProtocol::C(
            self.strings.tcl_version().expect("selected C9 name"),
        )
        .oo_property_option_name(original)
        .expect("selected C9 property option purpose")
        .selected()
        .to_vec()
    }

    /// Formatted accessor names use the original `CString` name extent.
    #[must_use]
    pub fn accessor_names(self, original: &[u8]) -> (Vec<u8>, Vec<u8>) {
        property_accessor_names(original, true)
    }

    /// Retain an absolute original object, or form the relative object namespace
    /// operand with its full counted name. This grants no namespace or cell.
    #[must_use]
    pub fn object_variable_operand(self, original: &[u8], namespace: &[u8]) -> Option<Vec<u8>> {
        property_object_variable_operand(original, namespace)
    }

    /// Resolve only the supplied ordered byte roster using its `CString` key.
    /// No original table header or index cache is supplied by this operation.
    ///
    /// # Errors
    /// Returns the native unknown or ambiguous property diagnostic.
    pub fn lookup(self, original: &[u8], members: &[&[u8]]) -> Result<usize, Vec<u8>> {
        property_name_lookup(original, members)
    }
}

fn property_definition_options() -> crate::native_index_lookup::NativeStaticIndexTable {
    crate::native_index_lookup::NativeStaticIndexTable::supported_backend(&[
        "-get", "-kind", "-set",
    ])
}

fn property_definition_kinds() -> crate::native_index_lookup::NativeStaticIndexTable {
    crate::native_index_lookup::NativeStaticIndexTable::supported_backend(&[
        "readable",
        "readwrite",
        "writable",
    ])
}

fn property_accessor_names(original: &[u8], declaration: bool) -> (Vec<u8>, Vec<u8>) {
    let name = tcl_core_types::c_string_extent(original);
    let make = |prefix: &[u8]| {
        let mut bytes = prefix.to_vec();
        if declaration {
            bytes.push(b'-');
        }
        bytes.extend_from_slice(name);
        bytes.push(b'>');
        bytes
    };
    (make(b"<ReadProp"), make(b"<WriteProp"))
}

fn property_object_variable_operand(original: &[u8], namespace: &[u8]) -> Option<Vec<u8>> {
    if tcl_core_types::c_string_extent(original).starts_with(b"::") {
        return None;
    }
    let mut qualified = namespace.to_vec();
    qualified.extend_from_slice(b"::");
    qualified.extend_from_slice(original);
    Some(qualified)
}

fn property_name_lookup(original: &[u8], members: &[&[u8]]) -> Result<usize, Vec<u8>> {
    tcl_cmd_core::prefix::OptionTable::abbreviating("property", members)
        .index_of(tcl_core_types::c_string_extent(original))
}

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
        property_declaration_validity(original)
    }
    /// Retain the actual ordered static table for property definition options.
    #[must_use]
    pub fn definition_options(self) -> crate::native_index_lookup::NativeStaticIndexTable {
        property_definition_options()
    }
    /// Retain the actual ordered static table for property access kinds.
    #[must_use]
    pub fn definition_kinds(self) -> crate::native_index_lookup::NativeStaticIndexTable {
        property_definition_kinds()
    }
    /// Select `TclOOLookupObjectVar`'s original operand producer. An absolute
    /// `CString` prefix retains the supplied header; a relative name makes one
    /// fresh header from the actual namespace and the full counted name.
    /// `None` means retain the original header, including its physical primary.
    #[must_use]
    pub fn object_variable_operand(self, original: &[u8], namespace: &[u8]) -> Option<Vec<u8>> {
        property_object_variable_operand(original, namespace)
    }
    /// Format accessor children from the original configuration member's
    /// `CString` extent. The member already contains the property's dash;
    /// declaration-name formatting is a separate pure name purpose.
    #[must_use]
    pub fn member_accessor_names(self, original: &[u8]) -> (Vec<u8>, Vec<u8>) {
        // naming.property.original-c9-counted-name-access
        // docs/design/analysis/name-resolution-proofs/property-original-c9-counted-name-access.md
        property_accessor_names(original, false)
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
        property_name_lookup(original, members)
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
    /// Select only the independent C9.0/C9.1 native name purpose. Older C,
    /// Jim, unsupported builds and absent native name issuers abstain.
    #[must_use]
    pub fn native_property_name_protocol(self) -> Option<NativePropertyNameProtocol> {
        let tcl_syntax::naming::NativeNameProtocol::C(version) = self.native_name_protocol()?
        else {
            return None;
        };
        (version >= TclVersion::V9_0).then_some(NativePropertyNameProtocol {
            strings: NativeStringProtocol::C(version),
        })
    }

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
    fn original_c9_property_validity_does_not_issue_a_physical_lookup_recipe() {
        // Implementation contract: naming.property.original-c9-declaration-validity
        // docs/design/analysis/name-resolution-proofs/property-original-c9-declaration-validity.md
        for version in TclVersion::ALL {
            let dialect = crate::InvocationDialect::for_version(version);
            let validity = native_property_declaration_validity(dialect, b"property");
            assert_eq!(validity.is_some(), version >= TclVersion::V9_0);
            assert_eq!(
                dialect.native_property_lookup_protocol().is_some(),
                version == TclVersion::V9_1
            );
            if validity.is_some() {
                assert_eq!(validity, Some(Ok(())));
                assert_eq!(
                    native_property_declaration_validity(dialect, b"p\xed\xa0\x80"),
                    Some(Ok(()))
                );
                for invalid in [b"-option".as_slice(), b"two words", b"a::b", b"a(b)"] {
                    assert!(
                        native_property_declaration_validity(dialect, invalid)
                            .unwrap()
                            .is_err()
                    );
                }
            }
        }
    }
    #[test]
    fn original_c9_name_recipe_matches_native_declaration_boundaries_without_cache_authority() {
        // naming.property.original-c9-counted-name-access
        // docs/design/analysis/name-resolution-proofs/property-original-c9-counted-name-access.md
        for (engine, captured) in [
            (
                "tcl9.0",
                include_str!("../tests/data/native_property_counted_original/9.0.4/stdout.tsv"),
            ),
            (
                "tcl9.1",
                include_str!("../tests/data/native_property_counted_original/9.1.0/stdout.tsv"),
            ),
        ] {
            let dialect = crate::InvocationDialect::of_profile(
                tcl_dialect::DialectProfile::find(engine).unwrap(),
            );
            let recipe = dialect.native_property_name_protocol().unwrap();
            assert_eq!(
                dialect.native_property_lookup_protocol().is_some(),
                engine == "tcl9.1"
            );
            for name in [
                b"p\xff".as_slice(),
                b"p\xed\xa0\x80",
                b"p\xed\xa0\x81",
                b"p\0tail",
            ] {
                assert_eq!(recipe.validate_declaration(name), Ok(()));
                assert_eq!(
                    recipe.dashed_name(name),
                    dialect
                        .native_name_protocol()
                        .unwrap()
                        .oo_property_option_name(name)
                        .unwrap()
                        .selected()
                );
            }
            for (index, name) in [
                b"p[bad]".as_slice(),
                b"p\\x",
                b"",
                b"-bad",
                b"p::q",
                b"p(q)",
            ]
            .into_iter()
            .enumerate()
            {
                let row = captured
                    .lines()
                    .find(|row| row.starts_with(&format!("INVALID_{index}|")))
                    .unwrap();
                let bytes: Vec<_> = row
                    .split('|')
                    .nth(3)
                    .unwrap()
                    .as_bytes()
                    .chunks_exact(2)
                    .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                    .collect();
                assert_eq!(recipe.validate_declaration(name), Err(bytes));
            }
            let names: &[&[u8]] = &[b"-p\xed\xa0\x80", b"-p\xed\xa0\x81", b"-p"];
            assert_eq!(recipe.lookup(b"-p\xed\xa0\x80", names), Ok(0));
            assert_eq!(recipe.lookup(b"-p\xed\xa0\x81", names), Ok(1));
            assert_eq!(recipe.lookup(b"-p\0tail", names), Ok(2));
            assert_eq!(
                recipe.accessor_names(b"p\0tail"),
                (b"<ReadProp-p>".to_vec(), b"<WriteProp-p>".to_vec())
            );
            assert_eq!(
                recipe.object_variable_operand(b"p\0tail", b"::oo::Obj1"),
                Some(b"::oo::Obj1::p\0tail".to_vec())
            );
        }
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "jimtcl", "f5-irules"] {
            let dialect = crate::InvocationDialect::of_profile(
                tcl_dialect::DialectProfile::find(engine).unwrap(),
            );
            assert!(
                dialect.native_property_name_protocol().is_none(),
                "{engine}"
            );
            assert!(
                dialect.native_property_lookup_protocol().is_none(),
                "{engine}"
            );
        }
    }
    #[test]
    fn original_c91_property_member_accessors_preserve_the_native_dash_once() {
        // naming.property.original-c9-counted-name-access
        // docs/design/analysis/name-resolution-proofs/property-original-c9-counted-name-access.md
        let dialect = crate::InvocationDialect::for_version(TclVersion::V9_1);
        let names = dialect.native_property_name_protocol().unwrap();
        let physical = dialect.native_property_lookup_protocol().unwrap();
        let captured =
            include_str!("../tests/data/native_property_counted_original/9.1.0/stdout.tsv");
        for (label, original) in [
            ("FF", b"p\xff".as_slice()),
            ("D800", b"p\xed\xa0\x80"),
            ("D801", b"p\xed\xa0\x81"),
            ("ZERO", b"p\0tail"),
        ] {
            let roster = captured
                .lines()
                .find(|row| row.starts_with(&format!("{label}_ROSTER|")))
                .unwrap();
            let member: Vec<_> = roster
                .split('|')
                .nth(3)
                .unwrap()
                .as_bytes()
                .chunks_exact(2)
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect();
            assert_eq!(member, names.dashed_name(original));
            assert_eq!(
                physical.member_accessor_names(&member),
                names.accessor_names(original)
            );
            assert!(
                captured
                    .lines()
                    .any(|row| row == format!("{label}_WRITE|0|0|"))
            );
            assert!(
                captured
                    .lines()
                    .any(|row| row == format!("{label}_READ|0|1|56414c5545"))
            );
        }
        assert_eq!(
            physical.member_accessor_names(b"-p\xff\0ignored"),
            (b"<ReadProp-p\xff>".to_vec(), b"<WriteProp-p\xff>".to_vec())
        );
    }

    #[test]
    fn property_temporary_lookup_and_mutation_axes_are_independent() {
        // Native proof: naming.property.original-foundation-epoch-and-cache
        // docs/design/analysis/name-resolution-proofs/property-original-foundation-epoch-and-cache.md
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
