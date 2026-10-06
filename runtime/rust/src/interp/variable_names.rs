// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Select the original variable operand's byte protocol before cell lookup.
use super::Interp;
use crate::frame::VarError;
use tcl_syntax::naming::{NativeNameProtocol, NativeVariableProjection};

impl Interp {
    pub(crate) fn require_variable_name_protocol(&self) -> Result<NativeNameProtocol, VarError> {
        let protocol = self.name_policy_protocol().map(|policy| policy.recipe());
        if self.namespaces.borrow().variable_name_protocol != protocol {
            self.namespaces.borrow_mut().variable_name_protocol = protocol;
        }
        protocol.ok_or_else(|| {
            self.clone().refuse_native_access(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "variable naming",
                ),
            );
            VarError::NameProtocolUnavailable
        })
    }

    pub(crate) fn variable_name_parts(
        &self,
        original: &[u8],
    ) -> Result<(Vec<u8>, Option<Vec<u8>>), VarError> {
        let input = self.combined_variable_input(original)?;
        Ok((
            input.root().selected().to_vec(),
            input.element().map(|element| element.selected().to_vec()),
        ))
    }

    pub(crate) fn combined_variable_input<'a>(
        &self,
        original: &'a [u8],
    ) -> Result<NativeVariableProjection<'a>, VarError> {
        Ok(self
            .require_variable_name_protocol()?
            .combined_variable_input(original))
    }

    pub(crate) fn separate_variable_input<'a>(
        &self,
        root: &'a [u8],
        element: Option<&'a [u8]>,
    ) -> Result<NativeVariableProjection<'a>, VarError> {
        Ok(self
            .require_variable_name_protocol()?
            .separate_variable_input(root, element))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interp::new_string;
    use tcl_dialect::TclVersion;
    use tcl_runtime_api::{FrameId, VarStore};

    #[test]
    fn original_root_qualification_precedes_byte_cell_lookup() {
        for version in TclVersion::ALL {
            let mut interp = Interp::new();
            interp.set_runtime_version(version);
            let value = new_string(b"value");
            interp.var_set(b"a\0z::b", value).unwrap();
            assert_eq!(interp.var_get(b"a\0z::b"), Some(value));
            assert_eq!(
                interp.var_get(b"a"),
                (version == TclVersion::V8_4).then_some(value)
            );
            assert_eq!(
                VarStore::get_bytes(&interp, FrameId(0), b"a\0z::b").unwrap(),
                Some(value)
            );
        }
    }

    #[test]
    fn combined_and_separate_element_extents_select_different_physical_keys() {
        for version in TclVersion::ALL {
            let mut interp = Interp::new();
            interp.set_runtime_version(version);
            let value = new_string(b"combined");
            interp.var_set_named(b"a(k\0tail)", value).unwrap();
            let selected = if version < TclVersion::V9_0 {
                b"k".as_slice()
            } else {
                b"k\0tail".as_slice()
            };
            assert_eq!(interp.var_get_elem(b"a", selected), Some(value));
            let separate = new_string(b"separate");
            interp.var_set_elem(b"b", b"k\0tail", separate).unwrap();
            let selected = if version == TclVersion::V8_4 {
                b"k".as_slice()
            } else {
                b"k\0tail".as_slice()
            };
            assert_eq!(interp.var_get_elem(b"b", selected), Some(separate));
            if version > TclVersion::V8_4 && version < TclVersion::V9_0 {
                assert_eq!(interp.var_get_elem(b"a", b"k\0tail"), None);
                assert_eq!(interp.var_get_elem(b"b", b"k"), None);
            }
        }
    }

    #[test]
    fn jim_namespace_and_scalar_keys_keep_original_nonunicode_bytes() {
        let mut interp = Interp::new();
        interp.set_dialect_profile(crate::environment::profile_for_dialect("jim"));
        let holder = crate::obj::Owned::fresh(new_string(b"ns\xff"));
        let namespace = interp
            .namespaces
            .borrow_mut()
            .retain_jim_namespace(holder, std::rc::Rc::from(&b"ns\xff"[..]));
        interp.current_ns.set(namespace);
        let value = new_string(b"exact");
        interp.var_set(b"a\0z::b", value).unwrap();
        assert_eq!(interp.var_get(b"a\0z::b"), Some(value));
        assert_eq!(interp.var_get(b"::ns\xff::a\0z::b"), Some(value));
        assert_eq!(interp.var_get(b"::ns\xc3\xbf::a\0z::b"), None);
    }
}
