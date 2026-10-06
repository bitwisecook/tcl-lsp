// SPDX-License-Identifier: AGPL-3.0-or-later
//! Closed native conformance drivers, enabled only by the `test-support` feature.
//! Engine `OriginalObject` and the supported C shim expose no dictionary search
//! cursor. These drivers retain the actual root/search inside the VM and reach
//! its physical operations through a registered native callback.
use crate::{NativeCommand, Value, Vm};
use std::rc::Rc;
use tcl_runtime_api::Completion;

/// Construct a sole-owned native C dictionary search mutation callback.
/// The callback mutates the same actual backing and advances its live search.
///
/// # Errors
/// Refuses an engine without the independently selected native C search API.
pub fn dictionary_search_mutation_driver(
    dialect: tcl_registry::InvocationDialect,
    add: bool,
) -> Result<Rc<dyn NativeCommand>, tcl_syntax::value::ValueError> {
    let protocol = dialect.native_string_protocol().ok_or(
        tcl_syntax::value::ValueError::CommandProtocolUnavailable("native C Dictionary search"),
    )?;
    let key = Value::string("k");
    let original_value = Value::string("V");
    let root = Value::dict(vec![(key.clone(), original_value.clone())]);
    let mut search = root.into_native_dictionary_search(protocol)?;
    drop(
        search
            .next_pair()?
            .expect("nonempty conformance dictionary"),
    );
    Ok(Rc::new(MutateSearchedBacking {
        search: std::cell::RefCell::new(search),
        original_value,
        key,
        protocol,
        add,
    }))
}

struct MutateSearchedBacking {
    search: std::cell::RefCell<crate::value::NativeDictionarySearch>,
    original_value: Value,
    key: Value,
    protocol: tcl_syntax::native_string::NativeStringProtocol,
    add: bool,
}
impl NativeCommand for MutateSearchedBacking {
    fn invoke(&self, vm: &mut Vm, args: &[Value]) -> Completion<Value> {
        assert!(args.is_empty());
        let progress = std::env::var_os("TCL_LSP_ORACLE_PROGRESS").as_deref()
            == Some(std::ffi::OsStr::new("1"));
        if progress {
            eprintln!(
                "oracle-callback dictionary-search operation=entry add={}",
                self.add
            );
        }
        let mut search = self.search.borrow_mut();
        assert_eq!(search.original_root().native_object_reference_count(), 1);
        assert_eq!(self.original_value.native_object_reference_count(), 2);
        let mut prepared = search
            .original_root()
            .prepare_native_dictionary(self.protocol)
            .unwrap();
        assert!(prepared.is_same_object(search.original_root()));
        let key = if self.add {
            Value::string("other")
        } else {
            self.key.clone()
        };
        prepared.set_member(key, Value::string("NEW")).unwrap();
        drop(prepared);
        if progress {
            eprintln!(
                "oracle-callback dictionary-search operation=mutation-complete add={}",
                self.add
            );
        }
        assert_eq!(
            self.original_value.native_object_reference_count(),
            if self.add { 2 } else { 1 }
        );
        let error = search.next_pair().unwrap_err();
        if progress {
            eprintln!(
                "oracle-callback dictionary-search operation=fatal-receipt add={}",
                self.add
            );
        }
        assert_eq!(error.native_access_refusal(), Some(
                tcl_syntax::raw_string::NativeValueAccessRefusal::FatalCondition(
                    tcl_syntax::raw_string::NativeFatalCondition::DictionarySearchConcurrentMutation)));
        crate::command::completion_from_cmd_error(vm, error.into())
    }
}
