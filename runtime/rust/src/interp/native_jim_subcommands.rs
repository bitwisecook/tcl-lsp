// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original namespace subcommand table, header and public-arity stages.

use super::{native_arguments::OriginalArguments, Code, Interp, OriginalJimLookupNamespace};
use crate::obj::{self, TclObj};
use tcl_cmd_core::CmdError;
use tcl_registry::native_jim_lookup::{
    NativeJimNamespaceSubcommand, NativeJimNamespaceSubcommandTable,
};
use tcl_syntax::value::ValueError;

pub(crate) enum OriginalJimNamespaceSubcommandOutcome {
    Invoke(&'static NativeJimNamespaceSubcommand),
    Complete(Code),
}

struct OriginalParserCommandList {
    original: obj::NativeObjectLifetime,
    fresh: bool,
}

impl Drop for OriginalParserCommandList {
    fn drop(&mut self) {
        if self.fresh {
            if let Ok(original) = self.original.checked_ptr() {
                // SAFETY: the memory-only lease pins this still-live private header.
                if unsafe { (*original).ref_count == 0 } {
                    // SAFETY: the refused or consumed private result has no native owner.
                    unsafe { obj::free_obj(original) };
                }
            }
        }
    }
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
        if argv.len() < 2 {
            let first = arguments.bytes_using(0, &check)?;
            let second = arguments.bytes_using(0, &check)?;
            self.set_error(&tcl_cmd_core::namespace::jim_namespace_missing_subcommand(
                &first, &second,
            ));
            check()?;
            return Ok(OriginalJimNamespaceSubcommandOutcome::Complete(Code::Error));
        }
        let original = arguments.original(1)?;
        let entry = if let Some(cache) = obj::native_jim_subcommand::cache(original) {
            table
                .cached_entry(&cache)
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "foreign original namespace subcommand cache",
                ))?
        } else {
            let special = dialect
                .native_jim_namespace_parser_comparison_table()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "original namespace parser literals",
                ))?;
            let help = self.native_jim_compare_immediate_checked(original, &special, 0, &check)?;
            if help && argv.len() == 2 {
                self.original_jim_namespace_top_usage(arguments, table, &check)?;
                return Ok(OriginalJimNamespaceSubcommandOutcome::Complete(Code::Ok));
            }
            let offset = if help { 2 } else { 1 };
            let selected = arguments.original(offset)?;
            if self.native_jim_compare_immediate_checked(selected, &special, 1, &check)? {
                let list = self.original_jim_namespace_command_list(table, b" ", &check)?;
                self.set_result(list.original.checked_ptr()?);
                check()?;
                return Ok(OriginalJimNamespaceSubcommandOutcome::Complete(Code::Ok));
            }
            let bytes = arguments.bytes_using(offset, &check)?;
            let selection = table.lookup_using(&bytes, &mut |index| {
                self.native_jim_compare_immediate_checked(selected, &table, index, &check)
            })?;
            let cache = match selection {
                Ok(cache) => cache,
                Err(failure) => {
                    if help {
                        self.original_jim_namespace_top_usage(arguments, table, &check)?;
                        return Ok(OriginalJimNamespaceSubcommandOutcome::Complete(Code::Ok));
                    }
                    let list = self.original_jim_namespace_command_list(table, b", ", &check)?;
                    let head = arguments.bytes_using(0, &check)?;
                    let member = arguments.bytes_using(offset, &check)?;
                    let names = self.native_object_string_bytes_using(
                        list.original.checked_ptr()?,
                        &check,
                        &CmdError::from,
                    )?;
                    self.set_error(
                        &tcl_cmd_core::namespace::jim_namespace_subcommand_diagnostic(
                            &head,
                            &member,
                            &names,
                            matches!(failure, tcl_cmd_core::prefix::Resolution::Ambiguous),
                        ),
                    );
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
            if help {
                self.original_jim_namespace_subcommand_usage(arguments, entry, &check, false)?;
                return Ok(OriginalJimNamespaceSubcommandOutcome::Complete(Code::Ok));
            }
            obj::native_jim_subcommand::install(original, cache, dialect, &mut || check())?;
            check()?;
            entry
        };
        if !entry.accepts_arguments(argv.len().saturating_sub(2)) {
            self.original_jim_namespace_subcommand_usage(arguments, entry, &check, true)?;
            return Ok(OriginalJimNamespaceSubcommandOutcome::Complete(Code::Error));
        }
        check()?;
        Ok(OriginalJimNamespaceSubcommandOutcome::Invoke(entry))
    }

    fn original_jim_namespace_top_usage(
        &mut self,
        arguments: &OriginalArguments,
        table: NativeJimNamespaceSubcommandTable,
        current: &impl Fn() -> Result<(), CmdError>,
    ) -> Result<(), CmdError> {
        let list = self.original_jim_namespace_command_list(table, b", ", current)?;
        let head = arguments.bytes_using(0, current)?;
        let names = self.native_object_string_bytes_using(
            list.original.checked_ptr()?,
            current,
            &CmdError::from,
        )?;
        self.set_result_bytes(&tcl_cmd_core::namespace::jim_namespace_subcommand_usage(
            &head, &names,
        ));
        current()
    }

    fn original_jim_namespace_command_list(
        &mut self,
        table: NativeJimNamespaceSubcommandTable,
        separator: &[u8],
        current: &impl Fn() -> Result<(), CmdError>,
    ) -> Result<OriginalParserCommandList, CmdError> {
        current()?;
        let dialect = self.native_invocation_dialect();
        let context = self.native_jim_object_context()?;
        let names: Vec<_> = table
            .entries()
            .iter()
            .map(|entry| obj::new_string_bytes(entry.name().as_bytes()))
            .collect();
        let list = obj::Owned::fresh(self.new_list_object(&names));
        let head = obj::Owned::fresh(obj::new_string_bytes(table.diagnostic_sort_head()));
        for original in [head.as_ptr(), list.as_ptr()] {
            crate::native_source::bind_context(original, &context)?;
            current()?;
        }
        let code = self.dispatch(&[head.as_ptr(), list.as_ptr()]);
        drop(head);
        drop(list);
        current()?;
        if code != Code::Ok {
            return Ok(OriginalParserCommandList {
                original: obj::NativeObjectLifetime::retain(self.result_obj()),
                fresh: false,
            });
        }
        let sorted = obj::NativeObjectLifetime::retain(self.result_obj());
        let sorted_current = || -> Result<(), CmdError> {
            current()?;
            sorted.checked_ptr()?;
            Ok(())
        };
        let strings =
            dialect
                .native_string_protocol()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "original Jim command-list String issuer",
                ))?;
        let members = crate::list::list_elements_native_using(
            sorted.checked_ptr()?,
            strings,
            &sorted_current,
            &CmdError::from,
        )?;
        let fresh = obj::new_string_bytes(b"");
        let joined = OriginalParserCommandList {
            original: obj::NativeObjectLifetime::retain(fresh),
            fresh: true,
        };
        crate::native_source::bind_context(fresh, &context)?;
        let append = dialect.native_object_append_protocol(None).ok_or(
            ValueError::CommandProtocolUnavailable("original Jim command-list append issuer"),
        )?;
        let operations = crate::value_ops::RuntimeAppendObjects {
            dialect,
            binary_recipe: None,
        };
        let receiver = crate::value_ops::RuntimeAppendValue::borrowed(fresh);
        let private_current = || -> Result<(), CmdError> {
            sorted_current()?;
            joined.original.checked_ptr()?;
            // SAFETY: the actual private receiver lease pins this live header.
            if unsafe { (*fresh).ref_count != 0 } || obj::is_shared(fresh) {
                return Err(ValueError::CommandProtocolUnavailable(
                    "original Jim private command-list receiver",
                )
                .into());
            }
            Ok(())
        };
        for (index, member) in members.iter().copied().enumerate() {
            let original = obj::NativeObjectLifetime::retain(member);
            let member_current = || -> Result<(), CmdError> {
                private_current()?;
                original.checked_ptr()?;
                Ok(())
            };
            // Reach the selected original updater with the caller's check at
            // every nested stage; the shared append then consumes that resident source.
            self.native_object_string_bytes_using(member, &member_current, &CmdError::from)?;
            let source = crate::value_ops::RuntimeAppendValue::borrowed(member);
            let mut first = None;
            let appended = tcl_cmd_core::native_append::append_unshared_jim_object_checked(
                &operations,
                append.recipe(),
                &receiver,
                &source,
                &mut || {
                    member_current().map_err(|error| {
                        first.get_or_insert(error);
                        ValueError::CommandProtocolUnavailable(
                            "original Jim command-list append owner",
                        )
                    })
                },
            );
            if let Some(error) = first {
                return Err(error);
            }
            appended?;
            if index + 1 < members.len() {
                let mut first = None;
                let appended = tcl_cmd_core::native_append::append_unshared_jim_cstring(
                    &operations,
                    append.recipe(),
                    &receiver,
                    separator,
                    &mut || {
                        private_current().map_err(|error| {
                            first.get_or_insert(error);
                            ValueError::CommandProtocolUnavailable(
                                "original Jim command-list separator owner",
                            )
                        })
                    },
                );
                if let Some(error) = first {
                    return Err(error);
                }
                appended?;
            }
        }
        private_current()?;
        Ok(joined)
    }

    fn original_jim_namespace_subcommand_usage(
        &mut self,
        arguments: &OriginalArguments,
        entry: &NativeJimNamespaceSubcommand,
        current: &impl Fn() -> Result<(), CmdError>,
        arity_error: bool,
    ) -> Result<(), CmdError> {
        current()?;
        let dialect = self.native_invocation_dialect();
        let protocol = dialect.native_object_append_protocol(None).ok_or(
            ValueError::CommandProtocolUnavailable("original Jim public arity append issuer"),
        )?;
        self.set_result_bytes(if arity_error {
            b"wrong # args: should be \""
        } else {
            b"Usage: "
        });
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
        if arity_error {
            pieces.push(b"\"");
        }
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
        if arity_error {
            self.set_error_state(b"NONE");
        }
        check()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::obj::Owned;

    const OBSERVED: &str =
        include_str!("../../tests/data/native_jim_namespace_subcommand/table-20261015.tsv");

    fn header(original: *mut TclObj) -> String {
        let descriptor = obj::obj_type_ptr(original);
        if descriptor.is_null() {
            "none".to_owned()
        } else {
            // SAFETY: a live original descriptor owns its static terminated name.
            unsafe { std::ffi::CStr::from_ptr((*descriptor).name) }
                .to_str()
                .unwrap()
                .to_owned()
        }
    }

    #[test]
    fn original_namespace_parser_specials_and_mutable_sort_match_retained_c_controls() {
        for (label, head, member, operands, sort) in [
            ("COMMANDS", "namespace", "-commands", &[][..], 0),
            (
                "COMMANDS_EXTRAS",
                "namespace",
                "-commands",
                &["84", "85"][..],
                0,
            ),
            ("HELP", "namespace", "-help", &[][..], 0),
            ("HELP_MEMBER", "namespace", "-help", &["upv"][..], 0),
            (
                "HELP_MEMBER_EXTRAS",
                "namespace",
                "-help",
                &["upv", "85"][..],
                0,
            ),
            ("HELP_COMMANDS", "namespace", "-help", &["-commands"][..], 0),
            (
                "HELP_MISSING",
                "namespace",
                "-help",
                &["unavailable"][..],
                0,
            ),
            ("RENAMED_MISSING", "renamed", "unavailable", &[][..], 0),
            ("RENAMED_EMPTY", "renamed", "", &[][..], 0),
            (
                "COMMANDS_REPLACED_SORT",
                "namespace",
                "-commands",
                &[][..],
                1,
            ),
            ("HELP_REPLACED_SORT", "namespace", "-help", &[][..], 1),
            (
                "MISSING_REPLACED_SORT",
                "renamed",
                "unavailable",
                &[][..],
                1,
            ),
            (
                "HELP_MEMBER_REPLACED_SORT",
                "namespace",
                "-help",
                &["upv"][..],
                1,
            ),
            ("COMMANDS_FAILED_SORT", "namespace", "-commands", &[][..], 2),
            ("HELP_FAILED_SORT", "namespace", "-help", &[][..], 2),
            (
                "MISSING_FAILED_SORT",
                "namespace",
                "unavailable",
                &[][..],
                2,
            ),
        ] {
            let mut interp = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect("jimtcl"),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            if sort != 0 {
                assert_eq!(
                    interp.eval_str(if sort == 1 {
                        b"proc lsort {args} {return {which canonical}}"
                    } else {
                        b"proc lsort {args} {return -code error SORTFAIL}"
                    }),
                    Code::Ok
                );
            }
            let context = interp.native_jim_object_context().unwrap();
            let originals: Vec<_> = std::iter::once(head)
                .chain(std::iter::once(member))
                .chain(operands.iter().copied())
                .map(|bytes| Owned::fresh(obj::new_string_bytes(bytes.as_bytes())))
                .collect();
            let argv: Vec<_> = originals.iter().map(Owned::as_ptr).collect();
            for original in &argv {
                crate::native_source::bind_context(*original, &context).unwrap();
            }
            for suffix in ["COLD", "REUSE"] {
                let phase = format!("{label}_{suffix}");
                interp.set_result_bytes(b"");
                let arguments = OriginalArguments::enter(&interp, &argv).unwrap();
                let code = match interp
                    .original_jim_namespace_subcommand(&argv, &arguments)
                    .unwrap()
                {
                    OriginalJimNamespaceSubcommandOutcome::Complete(code) => code,
                    OriginalJimNamespaceSubcommandOutcome::Invoke(_) => {
                        panic!("{phase}: special must complete in parser")
                    }
                };
                assert_eq!(arguments.finish(code), code, "{phase}");
                let result = interp.result_obj();
                for (kind, original) in [("MEMBER_AFTER", argv[1]), ("RESULT_HEADER", result)] {
                    assert!(
                        OBSERVED.lines().any(|line| line
                            == format!(
                                "{kind}|{phase}|{}|{}",
                                u8::from(obj::has_string_rep(original)),
                                header(original)
                            )),
                        "{kind}/{phase}"
                    );
                }
                let expected = OBSERVED
                    .lines()
                    .find_map(|line| {
                        line.strip_prefix(&format!("RESULT|{phase}|{}|", code.as_int()))
                    })
                    .unwrap();
                let bytes: Vec<_> = expected
                    .as_bytes()
                    .chunks_exact(2)
                    .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                    .collect();
                assert_eq!(obj::bytes_of(result), bytes, "{phase}");
            }
        }
    }

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
