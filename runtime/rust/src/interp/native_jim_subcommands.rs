// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original namespace subcommand table, header and public-arity stages.

use super::{native_arguments::OriginalArguments, Code, Interp, OriginalJimLookupNamespace};
use crate::obj::{self, TclObj};
use tcl_cmd_core::CmdError;
use tcl_registry::native_jim_lookup::NativeJimNamespaceSubcommand;
use tcl_syntax::value::ValueError;

pub(crate) enum OriginalJimNamespaceSubcommandOutcome {
    Invoke(&'static NativeJimNamespaceSubcommand),
    Complete(Code),
}

impl Interp {
    pub(crate) fn original_jim_namespace_subcommand(
        &mut self,
        argv: &[*mut TclObj],
        arguments: &OriginalArguments,
    ) -> Result<OriginalJimNamespaceSubcommandOutcome, CmdError> {
        let dialect = self.native_invocation_dialect();
        let table = dialect.native_jim_namespace_subcommand_table().ok_or(
            ValueError::CommandProtocolUnavailable("original namespace subcommand table issuer"),
        )?;
        let namespace = OriginalJimLookupNamespace::capture(self, self.current_ns())?;
        let actual = self.clone();
        let check = || namespace.check_current_arguments(&actual, arguments);
        check()?;
        let original = arguments.original(1)?;
        let entry = if let Some(cache) = obj::native_jim_subcommand::cache(original) {
            table
                .cached_entry(&cache)
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "foreign original namespace subcommand cache",
                ))?
        } else {
            let bytes = arguments.bytes_using(1, &check)?;
            for index in 0..table.entries().len() {
                if self.native_jim_compare_immediate_checked(original, &table, index, &check)? {
                    break;
                }
            }
            let cache = match table.lookup(&bytes) {
                Ok(cache) => cache,
                Err(_) => {
                    let names: Vec<_> = table.entries().iter().map(|entry| entry.name()).collect();
                    let selected = dialect.native_name_protocol().ok_or(
                        ValueError::CommandProtocolUnavailable(
                            "original namespace selector issuer",
                        ),
                    )?;
                    let written = selected.namespace_subcommand_input(&bytes);
                    self.set_error(&tcl_cmd_core::namespace::unknown_subcommand_message(
                        selected,
                        &names,
                        written.selected(),
                    ));
                    check()?;
                    return Ok(OriginalJimNamespaceSubcommandOutcome::Complete(Code::Error));
                }
            };
            let entry =
                table
                    .cached_entry(&cache)
                    .ok_or(ValueError::CommandProtocolUnavailable(
                        "original namespace selected table entry",
                    ))?;
            obj::native_jim_subcommand::install(original, cache, dialect, &mut || check())?;
            check()?;
            entry
        };
        if !entry.accepts_arguments(argv.len().saturating_sub(2)) {
            self.original_jim_namespace_subcommand_arity(arguments, entry, &check)?;
            return Ok(OriginalJimNamespaceSubcommandOutcome::Complete(Code::Error));
        }
        check()?;
        Ok(OriginalJimNamespaceSubcommandOutcome::Invoke(entry))
    }

    fn original_jim_namespace_subcommand_arity(
        &mut self,
        arguments: &OriginalArguments,
        entry: &NativeJimNamespaceSubcommand,
        current: &impl Fn() -> Result<(), CmdError>,
    ) -> Result<(), CmdError> {
        current()?;
        let dialect = self.native_invocation_dialect();
        let protocol = dialect.native_object_append_protocol(None).ok_or(
            ValueError::CommandProtocolUnavailable("original Jim public arity append issuer"),
        )?;
        self.set_result_bytes(b"wrong # args: should be \"");
        current()?;
        let published = self.result_obj();
        let lifetime = obj::NativeObjectLifetime::retain(published);
        let actual = self.clone();
        let check = || {
            current()?;
            lifetime.checked_ptr()?;
            if actual.result_obj() != published || obj::is_shared(published)
                // SAFETY: the checked original result lease pins this live header.
                || unsafe { (*published).ref_count != 1 }
            {
                return Err(ValueError::CommandProtocolUnavailable(
                    "original Jim public arity published receiver",
                )
                .into());
            }
            Ok(())
        };
        let head = arguments.bytes_using(0, &check)?;
        let operations = crate::value_ops::RuntimeAppendObjects {
            dialect,
            binary_recipe: None,
        };
        let receiver = crate::value_ops::RuntimeAppendValue::borrowed(published);
        let mut pieces: Vec<&[u8]> = vec![&head, b" ", entry.name().as_bytes()];
        if !entry.usage().is_empty() {
            pieces.extend([b" ".as_slice(), entry.usage().as_bytes()]);
        }
        pieces.push(b"\"");
        for piece in pieces {
            let mut first = None;
            let appended = tcl_cmd_core::native_append::append_published_unshared_jim_cstring(
                &operations,
                protocol.recipe(),
                &receiver,
                piece,
                &mut || {
                    check().map_err(|error| {
                        first.get_or_insert(error);
                        ValueError::CommandProtocolUnavailable("original Jim public arity owner")
                    })
                },
            );
            if let Some(error) = first {
                return Err(error);
            }
            appended?;
            check()?;
        }
        self.set_error_state(b"NONE");
        check()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::obj::Owned;

    const OBSERVED: &str =
        include_str!("../../tests/data/native_jim_namespace_subcommand/table-20261015.tsv");

    #[test]
    fn original_namespace_table_cache_precedes_public_arity_and_reuse_skips_string_getters() {
        let mut interp = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect("jimtcl"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        let context = interp.native_jim_object_context().unwrap();
        let head = Owned::fresh(obj::new_string_bytes(b"namespace"));
        let member = Owned::fresh(obj::new_string_bytes(b"parent"));
        let operand = Owned::fresh(obj::new_wide_int_obj(84));
        for original in [&head, &member, &operand] {
            crate::native_source::bind_context(original.as_ptr(), &context).unwrap();
        }
        let argv = [
            head.as_ptr(),
            member.as_ptr(),
            operand.as_ptr(),
            operand.as_ptr(),
        ];
        let arguments = OriginalArguments::enter(&interp, &argv).unwrap();
        assert!(matches!(
            interp
                .original_jim_namespace_subcommand(&argv, &arguments)
                .unwrap(),
            OriginalJimNamespaceSubcommandOutcome::Complete(Code::Error)
        ));
        let descriptor = obj::obj_type_ptr(member.as_ptr());
        // SAFETY: the live descriptor owns its static terminated type name.
        let member_name = unsafe { std::ffi::CStr::from_ptr((*descriptor).name) }
            .to_str()
            .unwrap();
        assert_eq!(member_name, "subcmd-lookup");
        let phase = "TABLE_parent_2_COLD";
        assert!(OBSERVED
            .lines()
            .any(|line| line == format!("MEMBER_AFTER|{phase}|1|{member_name}")));
        let result = interp.result_obj();
        let descriptor = obj::obj_type_ptr(result);
        // SAFETY: the published live result owns this static descriptor.
        let result_name = unsafe { std::ffi::CStr::from_ptr((*descriptor).name) }
            .to_str()
            .unwrap();
        assert!(OBSERVED
            .lines()
            .any(|line| line == format!("RESULT_HEADER|{phase}|1|{result_name}")));
        let expected = OBSERVED
            .lines()
            .find_map(|line| line.strip_prefix(&format!("RESULT|{phase}|1|")))
            .unwrap();
        let bytes: Vec<_> = expected
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect();
        assert_eq!(obj::bytes_of(result), bytes);
        assert_eq!(arguments.finish(Code::Error), Code::Error);
        obj::invalidate_string(member.as_ptr());
        let argv = [head.as_ptr(), member.as_ptr()];
        let arguments = OriginalArguments::enter(&interp, &argv).unwrap();
        match interp
            .original_jim_namespace_subcommand(&argv, &arguments)
            .unwrap()
        {
            OriginalJimNamespaceSubcommandOutcome::Invoke(entry) => {
                assert_eq!(entry.name(), "parent")
            }
            OriginalJimNamespaceSubcommandOutcome::Complete(_) => {
                panic!("matching genuine native cache must select parent")
            }
        }
        assert!(!obj::has_string_rep(member.as_ptr()));
        assert_eq!(arguments.finish(Code::Ok), Code::Ok);
    }
}
