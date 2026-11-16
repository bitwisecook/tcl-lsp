// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::counters;
use crate::interp::{Code, Interp};
use crate::obj::{self, TclObj};

fn leak_free(body: impl FnOnce(&mut Interp)) {
    counters::reset();
    {
        let mut interp = Interp::new();
        body(&mut interp);
    }
    assert_eq!(
        counters::finalize(),
        0,
        "residual: {} objs, {} bufs",
        counters::live_objs(),
        counters::live_bufs()
    );
    assert_eq!(counters::double_free_count(), 0);
}

#[test]
fn jim_child_constructor_refusal_preserves_typed_cause_and_parent_result() {
    // naming.interpreter.original-child-bootstrap-context
    // docs/design/analysis/name-resolution-proofs/interpreter-original-child-bootstrap-context.md
    // Direct constructor-worker ownership and unavailable-frame controls;
    // no native process or selected invocation after owner retirement claim.
    for retire_owner in [true, false] {
        counters::reset();
        {
            let mut interp = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect("jim"),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let argv = obj::Owned::fresh(obj::new_string_bytes(b"PARENT ARGV"));
            interp.var_set_at(b"argv", argv.as_ptr(), 0).unwrap();
            let word = obj::Owned::fresh(obj::new_string_bytes(b"interp"));
            interp
                .associate_native_jim_arguments(&[word.as_ptr()])
                .unwrap();
            let original = interp.native_jim_object_context().unwrap();
            interp.set_result_bytes(b"PARENT RESULT");
            let result = interp.result_obj();
            let expected = if retire_owner {
                original.retire();
                "retired Jim interpreter"
            } else {
                // Exact observed naming policy without entered event storage
                // refuses its global getter; it cannot donate a Jim parent cell.
                assert!(interp.set_observed_bigip_name_policy(
                    tcl_registry::f5::evidence::BigIpBuild::MEASURED_21_1_0_1,
                    tcl_registry::f5::BigIpExecutionContext::TmmIRule,
                    tcl_registry::f5::naming::BigIpNameEvent::HttpRequest,
                ));
                "observed event frame storage"
            };
            assert_eq!(
                super::interp_cmd(&mut interp, &[word.as_ptr()]),
                Code::Error
            );
            assert_eq!(
                interp.native_access_refusal(),
                Some(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        expected
                    )
                )
            );
            assert!(interp.host_refusal_pending());
            assert_eq!(interp.result_obj(), result);
            assert_eq!(interp.result_bytes(), b"PARENT RESULT");
            assert!(!interp.child_exists(b"::interp.handle0"));
            assert!(interp
                .find_command_id(crate::namespace::GLOBAL, b"::interp.handle0")
                .is_none());
        }
        assert_eq!(counters::finalize(), 0);
        assert_eq!(counters::double_free_count(), 0);
    }
}

#[test]
fn c_child_constructor_refusal_keeps_owner_cause_without_success_result() {
    // naming.interpreter.original-child-bootstrap-context
    // docs/design/analysis/name-resolution-proofs/interpreter-original-child-bootstrap-context.md
    // Direct C installer host controls, including a genuinely retained
    // nested owner; native success rows do not observe these host failures.
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for spelling in [None, Some(b"child".as_slice()), Some(b"outer child")] {
            counters::reset();
            {
                let mut interp = Interp::with_native_core(
                    crate::interp::default_host(),
                    crate::environment::profile_for_dialect(engine),
                    tcl_registry::special_vars::NativeBootstrapInputs::default(),
                )
                .unwrap();
                let select = |owner: &mut Interp| {
                    assert!(owner.set_observed_bigip_name_policy(
                        tcl_registry::f5::evidence::BigIpBuild::MEASURED_21_1_0_1,
                        tcl_registry::f5::BigIpExecutionContext::TmmIRule,
                        tcl_registry::f5::naming::BigIpNameEvent::HttpRequest,
                    ));
                };
                if spelling == Some(b"outer child".as_slice()) {
                    interp.create_child(Some(b"outer".to_vec()));
                    assert!(interp.child_exists(b"outer"));
                    interp.with_child(b"outer", select).unwrap();
                } else {
                    select(&mut interp);
                }
                let head = obj::Owned::fresh(obj::new_string_bytes(b"interp"));
                let member = obj::Owned::fresh(obj::new_string_bytes(b"create"));
                let path = spelling.map(|name| obj::Owned::fresh(obj::new_string_bytes(name)));
                let mut words = vec![head.as_ptr(), member.as_ptr()];
                if let Some(path) = &path {
                    words.push(path.as_ptr());
                }
                interp.set_result_bytes(b"PARENT RESULT");
                let result = interp.result_obj();
                assert_eq!(super::interp_create(&mut interp, &words), Code::Error);
                assert_eq!(
                        interp.native_access_refusal(),
                        Some(tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                            "unmeasured observed variable purpose",
                        ))
                    );
                assert!(interp.host_refusal_pending());
                assert_eq!(interp.result_obj(), result);
                assert_eq!(interp.result_bytes(), b"PARENT RESULT");
                if spelling == Some(b"outer child".as_slice()) {
                    let refusal = interp.native_access_refusal();
                    interp
                        .with_child(b"outer", |owner| {
                            assert!(owner.host_refusal_pending());
                            assert_eq!(owner.native_access_refusal(), refusal);
                            assert!(!owner.child_exists(b"child"));
                            assert!(owner
                                .find_command_id(crate::namespace::GLOBAL, b"child")
                                .is_none());
                        })
                        .unwrap();
                } else {
                    let name = spelling.unwrap_or(b"interp0");
                    assert!(!interp.child_exists(name));
                    assert!(interp
                        .find_command_id(crate::namespace::GLOBAL, name)
                        .is_none());
                }
            }
            assert_eq!(counters::finalize(), 0);
            assert_eq!(counters::double_free_count(), 0);
        }
    }
}

struct AliasCaptureHost {
    inner: std::rc::Rc<dyn tcl_platform::Host>,
    stdout: std::rc::Rc<std::cell::RefCell<Vec<u8>>>,
    stderr: std::rc::Rc<std::cell::RefCell<Vec<u8>>>,
}

impl tcl_platform::StdIo for AliasCaptureHost {
    fn write_stdout(&self, bytes: &[u8]) {
        self.stdout.borrow_mut().extend_from_slice(bytes);
    }
    fn write_stderr(&self, bytes: &[u8]) {
        self.stderr.borrow_mut().extend_from_slice(bytes);
    }
}

impl tcl_platform::Host for AliasCaptureHost {
    fn capabilities(&self) -> tcl_platform::Capabilities {
        self.inner.capabilities()
    }
    fn clock(&self) -> &dyn tcl_platform::Clock {
        self.inner.clock()
    }
    fn stdio(&self) -> &dyn tcl_platform::StdIo {
        self
    }
    fn env(&self) -> &dyn tcl_platform::Env {
        self.inner.env()
    }
    fn numeric_environment(&self) -> Option<&dyn tcl_platform::NumericEnvironment> {
        self.inner.numeric_environment()
    }
    fn native_integer_formatter(&self) -> Option<&dyn tcl_platform::NativeIntegerFormatter> {
        self.inner.native_integer_formatter()
    }
    fn system_encoding(&self) -> tcl_platform::SystemEncoding {
        self.inner.system_encoding()
    }
    fn filesystem(&self) -> Option<&dyn tcl_platform::Filesystem> {
        self.inner.filesystem()
    }
    fn sockets(&self) -> Option<&dyn tcl_platform::Sockets> {
        self.inner.sockets()
    }
    fn process(&self) -> Option<&dyn tcl_platform::Process> {
        self.inner.process()
    }
}

fn compare_alias_publication_fixture(engine: &str, source: &[u8], native: &[u8]) {
    counters::reset();
    {
        let stdout = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let stderr = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let host = std::rc::Rc::new(AliasCaptureHost {
            inner: crate::interp::default_host(),
            stdout: std::rc::Rc::clone(&stdout),
            stderr: std::rc::Rc::clone(&stderr),
        });
        let profile = tcl_registry::model::resolve_environment(engine).unit_profile();
        let mut interp = Interp::with_native_core(
            host,
            profile,
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .expect("selected original native interpreter constructor");
        crate::cmd_proc::install_stock_scripted_wrappers(&mut interp);
        let code = interp.eval_str(source);
        assert!(
                !interp.host_refusal_pending(),
                "{engine}: admission {:?}, native access {:?}, execution {:?}, completed original rows {:?}",
                interp.native_compilation_admission_error(),
                interp.native_access_refusal(),
                interp.native_execution_refusal(),
                stdout.borrow().as_slice()
            );
        assert_eq!(code, Code::Ok, "{engine}: {:?}", interp.result_bytes());
        assert!(stderr.borrow().is_empty(), "{engine}: guest stderr");
        let stdout = stdout.borrow();
        // Provider version metadata is outside the public alias control rows.
        let observed = stdout
            .splitn(2, |byte| *byte == b'\n')
            .nth(1)
            .expect("Runtime version row");
        let expected = native
            .splitn(2, |byte| *byte == b'\n')
            .nth(1)
            .expect("native version row");
        assert_eq!(observed, expected, "{engine}: original public alias rows");
    }
    assert_eq!(
        counters::finalize(),
        0,
        "residual: {} objs, {} bufs",
        counters::live_objs(),
        counters::live_bufs()
    );
    assert_eq!(counters::double_free_count(), 0);
}

// Native proof: naming.alias.c-current-namespace-publication-holder
// docs/design/analysis/name-resolution-proofs/alias-c-current-namespace-publication-holder.md
#[test]
fn alias_publication_uses_current_c_holder_and_preserves_jim_incompatible_api() {
    let source = include_bytes!(
        "../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/probe.tcl"
    );
    for (engine, native) in [
        (
            "tcl8.4",
            include_bytes!(
                "../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/8.4.20/stdout"
            )
            .as_slice(),
        ),
        (
            "tcl8.5",
            include_bytes!(
                "../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/8.5.19/stdout"
            )
            .as_slice(),
        ),
        (
            "tcl8.6",
            include_bytes!(
                "../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/8.6.18/stdout"
            )
            .as_slice(),
        ),
        (
            "tcl9.0",
            include_bytes!(
                "../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/9.0.4/stdout"
            )
            .as_slice(),
        ),
        (
            "tcl9.1",
            include_bytes!(
                "../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/9.1.0/stdout"
            )
            .as_slice(),
        ),
        (
            "jim",
            include_bytes!(
                "../../../../rust/tcl-registry/tests/data/native_c_alias_holder197/jim/stdout"
            )
            .as_slice(),
        ),
    ] {
        compare_alias_publication_fixture(engine, source, native);
    }
}

// Native proof: naming.alias.original-child-interpreter-publication
// docs/design/analysis/name-resolution-proofs/alias-original-child-interpreter-publication.md
#[test]
fn jim_child_alias_result_is_born_in_the_actual_parent_allocator() {
    use crate::interp::native_arguments::OriginalArguments;
    use crate::interp::native_operation_currency::NativeOperationScope;
    let mut interp = Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("jim"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    assert_eq!(interp.eval_str(b"set child [interp]"), Code::Ok);
    let child = interp.result_bytes();
    let _parent_scope = NativeOperationScope::enter(&interp).unwrap();
    let owned = [child.as_slice(), b"alias", b"::A::a", b"list", b"VALUE"]
        .map(|bytes| crate::obj::Owned::fresh(crate::obj::new_string_bytes(bytes)));
    let argv = owned
        .iter()
        .map(crate::obj::Owned::as_ptr)
        .collect::<Vec<_>>();
    let arguments = OriginalArguments::enter(&interp, &argv).unwrap();
    assert_eq!(
        super::child_alias_handle_original(&mut interp, &child, &argv, &arguments),
        Code::Ok,
    );
    let parent = interp.native_jim_object_context().unwrap();
    let receiver = interp
        .with_child(&child, |receiver| receiver.clone())
        .expect("actual entered child interpreter");
    let receiver_context = receiver.native_jim_object_context().unwrap();
    let result = interp.result_obj();
    crate::obj::validate_jim_allocator(result, &parent.allocation_pool()).unwrap();
    assert!(
        crate::obj::validate_jim_allocator(result, &receiver_context.allocation_pool(),).is_err()
    );
    crate::native_source::bind_context(result, &parent).unwrap();
    assert!(crate::native_source::bind_context(result, &receiver_context).is_err());
    assert!(interp.result_bytes().is_empty());
    assert!(!interp.host_refusal_pending());
    assert!(!receiver.host_refusal_pending());
}

// Native proof: naming.alias.original-child-interpreter-publication
// docs/design/analysis/name-resolution-proofs/alias-original-child-interpreter-publication.md
#[test]
fn child_alias_publication_matches_original_supported_provider_apis() {
    let source = include_bytes!(
        "../../../../rust/tcl-registry/tests/data/native_child_alias_publication199/probe.tcl"
    );
    for (engine, native) in [
            ("tcl8.4", include_bytes!("../../../../rust/tcl-registry/tests/data/native_child_alias_publication199/8.4.20/stdout").as_slice()),
            ("tcl8.5", include_bytes!("../../../../rust/tcl-registry/tests/data/native_child_alias_publication199/8.5.19/stdout").as_slice()),
            ("tcl8.6", include_bytes!("../../../../rust/tcl-registry/tests/data/native_child_alias_publication199/8.6.18/stdout").as_slice()),
            ("tcl9.0", include_bytes!("../../../../rust/tcl-registry/tests/data/native_child_alias_publication199/9.0.4/stdout").as_slice()),
            ("tcl9.1", include_bytes!("../../../../rust/tcl-registry/tests/data/native_child_alias_publication199/9.1.0/stdout").as_slice()),
            ("jim", include_bytes!("../../../../rust/tcl-registry/tests/data/native_child_alias_publication199/jim/stdout").as_slice()),
        ] {
            compare_alias_publication_fixture(engine, source, native);
        }
}

// Native proof: naming.jim.rooted-alias-publication-and-lookup
// docs/design/analysis/name-resolution-proofs/jim-rooted-alias-publication-and-lookup.md
#[test]
fn jim_alias_owner_replaces_canonical_root_slots_and_keeps_local_competition() {
    counters::reset();
    {
        let profile = tcl_registry::model::resolve_environment("jim").unit_profile();
        let mut interp = Interp::with_native_core(
            crate::interp::default_host(),
            profile,
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .expect("selected original Jim core constructor");
        let observe = |interp: &mut Interp, source: &[u8]| {
            assert_eq!(
                interp.eval_str(source),
                Code::Ok,
                "{:?}",
                interp.result_bytes()
            );
            interp.result_bytes()
        };
        // This API control isolates publication from the standalone entry.
        // The unchanged standalone source is compared independently below.
        for (original, value, setup, observation, expected) in [
                (
                    b"::alias_slot".as_slice(),
                    b"ROOT_ALIAS".as_slice(),
                    b"proc ::alias_slot {} {return ORIGINAL}".as_slice(),
                    b"set a [catch {alias_slot} ar];set b [catch {::alias_slot} br];list $a $ar $b $br [catch {info body ::alias_slot}]".as_slice(),
                    b"0 ROOT_ALIAS 0 ROOT_ALIAS 1".as_slice(),
                ),
                (
                    b"::AliasProbe slot".as_slice(),
                    b"MULTIWORD_ALIAS".as_slice(),
                    b"proc {::AliasProbe slot} {} {return ORIGINAL_MULTIWORD}".as_slice(),
                    b"set a [catch {{AliasProbe slot}} ar];set b [catch {{::AliasProbe slot}} br];list $a $ar $b $br [catch {info body {::AliasProbe slot}}]".as_slice(),
                    b"0 MULTIWORD_ALIAS 0 MULTIWORD_ALIAS 1".as_slice(),
                ),
                (
                    b"::::colon_slot".as_slice(),
                    b"COLON_ALIAS".as_slice(),
                    b"".as_slice(),
                    b"set a [catch {colon_slot} ar];set b [catch {::colon_slot} br];set c [catch {::::colon_slot} cr];list $a $ar $b $br $c $cr".as_slice(),
                    b"0 COLON_ALIAS 0 COLON_ALIAS 0 COLON_ALIAS".as_slice(),
                ),
                (
                    b"::competition_slot".as_slice(),
                    b"GLOBAL_ALIAS".as_slice(),
                    b"proc competition_slot {} {return ORIGINAL_GLOBAL};namespace eval AliasContext {proc competition_slot {} {return LOCAL_PROC}}".as_slice(),
                    b"set a [catch {competition_slot} ar];set b [catch {namespace eval AliasContext {competition_slot}} br];set c [catch {namespace eval AliasContext {::competition_slot}} cr];list $a $ar $b $br $c $cr".as_slice(),
                    b"0 GLOBAL_ALIAS 0 LOCAL_PROC 0 GLOBAL_ALIAS".as_slice(),
                ),
            ] {
                observe(&mut interp, setup);
                interp.install_alias(original, b"list".to_vec(), vec![value.to_vec()]).expect("selected original alias publication owner");
                assert_eq!(observe(&mut interp, observation), expected, "{original:?}");
            }
    }
    assert_eq!(
        counters::finalize(),
        0,
        "residual: {} objs, {} bufs",
        counters::live_objs(),
        counters::live_bufs()
    );
    assert_eq!(counters::double_free_count(), 0);
}

#[test]
fn original_jim_core_alias_matches_all_four_standalone_native_public_rows() {
    // naming.jim.rooted-alias-publication-and-lookup
    // docs/design/analysis/name-resolution-proofs/jim-rooted-alias-publication-and-lookup.md
    // Core registration and unchanged public source; no private object
    // identity follows from the native stdout observations.
    compare_alias_publication_fixture(
        "jim",
        include_bytes!(
            "../../../../rust/tcl-registry/tests/data/native_jim_alias_publication194/probe.tcl"
        ),
        include_bytes!(
            "../../../../rust/tcl-registry/tests/data/native_jim_alias_publication194/jim/stdout"
        ),
    );
}

#[test]
fn jim_core_alias_retains_original_name_result_and_prefix_members() {
    // naming.alias.jim-original-core-prefix-object-storage
    // docs/design/analysis/name-resolution-proofs/alias-jim-original-core-prefix-object-storage.md
    // The original API observer independently records name/member identity
    // and absent resident member strings at definition, query and invocation.
    // Implementation control: the retained prefix owner supplies actual
    // objects to dispatch; native public rows above establish byte results.
    counters::reset();
    {
        let profile = tcl_registry::model::resolve_environment("jim").unit_profile();
        let mut interp = Interp::with_native_core(
            crate::interp::default_host(),
            profile,
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        let head = obj::Owned::fresh(obj::new_string_bytes(b"alias"));
        let name = obj::Owned::fresh(obj::new_string_bytes(b"::original_prefix_alias"));
        let target = obj::Owned::fresh(obj::new_string_bytes(b"list"));
        let member = obj::Owned::fresh(obj::new_wide_int_obj(17));
        assert!(!obj::has_string_rep(member.as_ptr()));
        assert_eq!(
            interp.dispatch(&[
                head.as_ptr(),
                name.as_ptr(),
                target.as_ptr(),
                member.as_ptr()
            ]),
            Code::Ok
        );
        assert_eq!(interp.get_obj_result(), name.as_ptr());
        assert!(!obj::has_string_rep(member.as_ptr()));
        let info = obj::Owned::fresh(obj::new_string_bytes(b"info"));
        let selector = obj::Owned::fresh(obj::new_string_bytes(b"alias"));
        assert_eq!(
            interp.dispatch(&[info.as_ptr(), selector.as_ptr(), name.as_ptr()]),
            Code::Ok
        );
        let query = interp.get_obj_result();
        assert_eq!(
            tcl_syntax::value::ValueOps::list_elements(&mut interp, &query).unwrap(),
            vec![target.as_ptr(), member.as_ptr()]
        );
        assert!(!obj::has_string_rep(member.as_ptr()));
        // Current owner control: a second public query keeps this header;
        // the native273 observer separately proves member identity/cache.
        assert_eq!(
            interp.dispatch(&[info.as_ptr(), selector.as_ptr(), name.as_ptr()]),
            Code::Ok
        );
        assert_eq!(interp.get_obj_result(), query);
        assert!(!obj::has_string_rep(member.as_ptr()));
        assert_eq!(interp.dispatch(&[name.as_ptr()]), Code::Ok);
        assert!(!obj::has_string_rep(member.as_ptr()));
        let result = interp.get_obj_result();
        assert_eq!(
            tcl_syntax::value::ValueOps::list_elements(&mut interp, &result).unwrap(),
            vec![member.as_ptr()]
        );
        let c = Interp::new();
        assert!(c.resolve_cmd_token(b"alias").is_none());
    }
    assert_eq!(counters::finalize(), 0);
    assert_eq!(counters::double_free_count(), 0);
}

#[test]
fn jim_child_counted_crossings_match_original_native_public_rows() {
    // naming.interpreter.jim-original-object-crossing
    // docs/design/analysis/name-resolution-proofs/jim-original-object-crossing.md
    // The counted driver metadata/result channel is separate from the
    // guest's public rows. These rows assert no private pointer ownership.
    let recorded = include_bytes!(
            "../../../../rust/tcl-registry/tests/data/native_jim_child_object_crossing302/jim/original-counted-child-crossing/stdout"
        );
    let mut channels = recorded.splitn(3, |byte| *byte == b'\n');
    assert_eq!(
        channels.next().unwrap(),
        b"VERSION|0|302e38342d392d6735626163376339"
    );
    assert_eq!(channels.next().unwrap(), b"ORIGINAL|0|");
    let guest_rows = channels.next().unwrap();
    compare_alias_publication_fixture(
            "jim",
            include_bytes!(
                "../../../../rust/tcl-registry/tests/data/native_jim_child_object_crossing302/probe.tcl"
            ),
            guest_rows,
        );
}

/// The `interp` ensemble and the child-as-command dispatch
/// are `Tcl_GetIndexFromObj(…, "option", 0)` tables (`options[]` in
/// `Tcl_InterpObjCmd` and `NRChildCmd`, `tclInterp.c`), so subcommands
/// abbreviate and the empty word — a prefix of every entry — is
/// `ambiguous option ""`. The `interp` list still names only what this
/// runtime dispatches; the child list is tclsh's in full.
///
/// tclsh 8.6.16 / 9.0.4 (the verdicts, not the shortened `interp` list):
///   interp {}       -> ambiguous option "": must be …
///   interp c j      -> ambiguous option "c": must be …
///   interp cr j     -> j
///   interp e {}     -> ambiguous option "e": must be …
///   interp sl       -> the children list (8.x's deprecated spelling)
///   kid ev {set x 1} -> 1   ;  kid h / kid hi -> ambiguous option "h" / "hi"
///   kid x           -> bad option "x": must be alias, aliases, bgerror,
///                      debug, eval, expose, hide, hidden, issafe,
///                      invokehidden, limit, marktrusted, or recursionlimit
#[test]
fn jim_interpreter_handle_factory_alias_frame_and_deletion() {
    let mut i = Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("jim"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .expect("actual original Jim core constructor");
    assert_eq!(i.eval_str(br#"set ::x GLOBAL; namespace eval ::N {proc observe {} {set x LOCAL; set child [interp]; $child alias readX set x; $child alias where namespace current; set result [list [$child eval {readX}] [$child eval {where}]]; $child delete; return $result}}; ::N::observe"#), Code::Ok, "result={:?}, admission={:?}, access={:?}", i.result_bytes(), i.native_compilation_admission_error(), i.native_access_refusal());
    assert_eq!(i.result_bytes(), b"LOCAL ::N");
}

#[test]
fn jim_interpreter_handle_isolation() {
    let mut i = Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("jim"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .expect("actual original Jim core constructor");
    assert_eq!(i.eval_str(br#"namespace eval ::N {proc probe {} {return PARENT}}; package provide Isolated 2.0; set child [interp]; $child eval {namespace eval ::N {proc probe {} {return CHILD}}}; set result [list [::N::probe] [$child eval {::N::probe}] [$child eval {catch {package require Isolated}}]]; $child delete; set result"#), Code::Ok, "result={:?}, admission={:?}, access={:?}", i.result_bytes(), i.native_compilation_admission_error(), i.native_access_refusal());
    assert_eq!(i.result_bytes(), b"PARENT CHILD 1");
}

#[test]
fn interp_abbreviated_operations_dispatch_in_root_and_child() {
    leak_free(|i| {
        assert_eq!(i.eval_str(b"interp cr j"), Code::Ok);
        assert_eq!(i.result_bytes(), b"j");
        assert_eq!(i.eval_str(b"interp ev {} {set x 1}"), Code::Ok);
        assert_eq!(i.result_bytes(), b"1");
        // The primary lookup table retains this spelling independently of
        // the release-specific table used to report a failed lookup.
        assert_eq!(i.eval_str(b"llength [interp sl]"), Code::Ok);
        assert_eq!(i.result_bytes(), b"1");
        assert_eq!(i.eval_str(b"interp create kid"), Code::Ok);
        assert_eq!(i.eval_str(b"kid ev {set x 1}"), Code::Ok);
        assert_eq!(i.result_bytes(), b"1");
    });
}

/// `interp create`'s and `interp invokehidden`'s leading
/// options are `Tcl_GetIndexFromObj(…, "option", 0)` tables
/// (`createOptions[]` / `hiddenOptions[]`, `tclInterp.c`), so they
/// abbreviate and the lone `-` — a prefix of every entry — is `ambiguous`.
///
/// tclsh 8.6.16 / 9.0.4:
///   interp create -x k         -> bad option "-x": must be -safe or --
///   interp create - k          -> ambiguous option "-": must be -safe or --
///   interp create -s k         -> k
///   interp invokehidden i -x f -> bad option "-x": must be -global, -namespace, or --
///   interp invokehidden i - f  -> ambiguous option "-": must be -global, -namespace, or --
#[test]
fn hidden_options_refuse_namespace_on_the_original_c84_table() {
    for version in tcl_dialect::TclVersion::ALL {
        let mut interp = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect(version.dialect_profile_name()),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        assert_eq!(interp.eval_str(b"interp create kid"), Code::Ok);
        assert_eq!(
            interp.eval_str(b"kid eval {interp hide {} set hiddenSet}"),
            Code::Ok
        );
        let code = interp.eval_str(b"kid invokehidden -namespace chosen hiddenSet item CHILD");
        if version == tcl_dialect::TclVersion::V8_4 {
            assert_eq!(code, Code::Error);
            assert_eq!(
                interp.result_bytes(),
                b"bad option \"-namespace\": must be -global or --"
            );
        } else {
            assert_eq!(code, Code::Ok);
            assert_eq!(interp.result_bytes(), b"CHILD");
            assert_eq!(
                interp.eval_str(b"kid invokehidden hiddenSet ::chosen::item"),
                Code::Ok
            );
            assert_eq!(interp.result_bytes(), b"CHILD");
            assert_eq!(interp.eval_str(b"kid eval {info exists ::item}"), Code::Ok);
            assert_eq!(interp.result_bytes(), b"0");
        }
    }
}

#[test]
fn interp_create_and_invokehidden_options_resolve_like_tcl_get_index_from_obj() {
    const CREATE_MUST: &str = "must be -safe or --";
    const HIDDEN_MUST: &str = "must be -global, -namespace, or --";
    leak_free(|i| {
        let err = |i: &mut Interp, script: &[u8]| {
            assert_eq!(i.eval_str(script), Code::Error, "expected an error");
            String::from_utf8_lossy(&i.result_bytes()).into_owned()
        };
        assert_eq!(
            err(i, b"interp create -x k"),
            format!("bad option \"-x\": {CREATE_MUST}")
        );
        assert_eq!(
            err(i, b"interp create - k"),
            format!("ambiguous option \"-\": {CREATE_MUST}")
        );
        assert_eq!(i.eval_str(b"interp create -s k"), Code::Ok);
        assert_eq!(i.result_bytes(), b"k");
        assert_eq!(i.eval_str(b"interp create -- k2"), Code::Ok);
        assert_eq!(i.result_bytes(), b"k2");
        assert_eq!(i.eval_str(b"interp create kid"), Code::Ok);
        assert_eq!(
            err(i, b"interp invokehidden kid -x foo"),
            format!("bad option \"-x\": {HIDDEN_MUST}")
        );
        assert_eq!(
            err(i, b"interp invokehidden kid - foo"),
            format!("ambiguous option \"-\": {HIDDEN_MUST}")
        );
    });
}

#[test]
fn original_interpreter_paths_preserve_child_keys_and_create_result_objects() {
    // Native proof: naming.interpreter.counted-list-path-cstring-child-keys
    // docs/design/analysis/name-resolution-proofs/interpreter-counted-list-path-cstring-child-keys.md
    // Native proof: naming.interpreter.singleton-create-original-spelling
    // docs/design/analysis/name-resolution-proofs/interpreter-singleton-create-original-spelling.md
    // Native proof: naming.interpreter.malformed-path-list-error
    // docs/design/analysis/name-resolution-proofs/interpreter-malformed-path-list-error.md
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interp = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect(engine),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        let call = |interp: &mut Interp, member: &[u8], path: *mut TclObj| {
            let head = obj::Owned::fresh(obj::new_string_bytes(b"interp"));
            let member = obj::Owned::fresh(obj::new_string_bytes(member));
            interp.dispatch(&[head.as_ptr(), member.as_ptr(), path])
        };
        for bytes in [
            b"child\0suffix".as_slice(),
            b"opaque\xff",
            b"surrogate\xed\xa0\x80",
            b"encoded\xc0\x80tail",
        ] {
            let original = obj::Owned::fresh(obj::new_string_bytes(bytes));
            assert_eq!(
                call(&mut interp, b"create", original.as_ptr()),
                Code::Ok,
                "{engine}"
            );
            assert_eq!(interp.get_obj_result(), original.as_ptr());
            assert_eq!(call(&mut interp, b"exists", original.as_ptr()), Code::Ok);
            assert_eq!(interp.result_bytes(), b"1");
            assert_eq!(call(&mut interp, b"delete", original.as_ptr()), Code::Ok);
            assert_eq!(call(&mut interp, b"exists", original.as_ptr()), Code::Ok);
            assert_eq!(interp.result_bytes(), b"0");
        }
        let singleton = obj::Owned::fresh(obj::new_string_bytes(b"{with space}"));
        assert_eq!(call(&mut interp, b"create", singleton.as_ptr()), Code::Ok);
        assert_eq!(interp.get_obj_result(), singleton.as_ptr());
        assert_eq!(call(&mut interp, b"exists", singleton.as_ptr()), Code::Ok);
        assert_eq!(interp.result_bytes(), b"0");
        let address = obj::Owned::fresh(obj::new_string_bytes(b"{{with space}}"));
        assert_eq!(call(&mut interp, b"exists", address.as_ptr()), Code::Ok);
        assert_eq!(interp.result_bytes(), b"1");
        let empty = obj::Owned::fresh(obj::new_string_bytes(b""));
        assert_eq!(call(&mut interp, b"create", empty.as_ptr()), Code::Ok);
        assert_eq!(interp.get_obj_result(), empty.as_ptr());
        let empty_child = obj::Owned::fresh(obj::new_string_bytes(b"{}"));
        assert_eq!(call(&mut interp, b"exists", empty_child.as_ptr()), Code::Ok);
        assert_eq!(interp.result_bytes(), b"1");
        assert_eq!(call(&mut interp, b"delete", empty_child.as_ptr()), Code::Ok);
        assert_eq!(call(&mut interp, b"exists", empty_child.as_ptr()), Code::Ok);
        assert_eq!(interp.result_bytes(), b"0");
        let malformed = obj::Owned::fresh(obj::new_string_bytes(b"{"));
        assert_eq!(
            call(&mut interp, b"create", malformed.as_ptr()),
            Code::Error
        );
        assert_eq!(call(&mut interp, b"exists", malformed.as_ptr()), Code::Ok);
        assert_eq!(interp.result_bytes(), b"0");
        assert!(!interp.host_refusal_pending());
    }
}

// Needs the numeric tower: the child's `dbl` proc computes via `expr`.
#[cfg(have_tommath)]
#[test]
fn child_interpreters() {
    // `interp create`/`eval`/`exists`/`children`/`delete` + the child as a
    // command (`$child eval …`). Verified vs tclsh 9.0.
    leak_free(|i| {
        assert_eq!(i.eval_str(b"interp create kid"), Code::Ok);
        assert_eq!(i.result_bytes(), b"kid");
        assert_eq!(i.eval_str(b"interp exists kid"), Code::Ok);
        assert_eq!(i.result_bytes(), b"1");
        // The child is isolated and addressable as a command.
        i.eval_str(b"kid eval {set x 42; proc dbl n {expr {$n*2}}}");
        assert_eq!(i.eval_str(b"kid eval {dbl $x}"), Code::Ok);
        assert_eq!(i.result_bytes(), b"84");
        // ... and via `interp eval`.
        assert_eq!(i.eval_str(b"interp eval kid {set x}"), Code::Ok);
        assert_eq!(i.result_bytes(), b"42");
        // The parent doesn't see the child's variable.
        assert_eq!(i.eval_str(b"info exists x"), Code::Ok);
        assert_eq!(i.result_bytes(), b"0");
        assert_eq!(i.eval_str(b"interp children"), Code::Ok);
        assert_eq!(i.result_bytes(), b"kid");
        // Children get the predefined globals.
        assert_eq!(
            i.eval_str(b"kid eval {set ::tcl_platform(platform)}"),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"unix");
        // Delete removes the child and its command.
        assert_eq!(i.eval_str(b"interp delete kid"), Code::Ok);
        assert_eq!(i.eval_str(b"interp exists kid"), Code::Ok);
        assert_eq!(i.result_bytes(), b"0");
        assert_eq!(i.eval_str(b"kid eval {set x}"), Code::Error);
    });
}

// Needs the numeric tower: alias targets are `expr`-backed (`padd`) and `::tcl::mathop::*`.
#[cfg(have_tommath)]
#[test]
fn cross_interp_aliases() {
    // A child alias delegating to a parent command (both syntaxes), and
    // re-entrancy (a parent alias re-entering the *same* child mid-eval —
    // the Safe Base pattern — now recurses correctly, bounded not forbidden).
    leak_free(|i| {
        i.eval_str(b"proc padd {a b} {expr {$a+$b}}");
        i.eval_str(b"set c [interp create]");
        // `interp alias $c name {} target prefix...`
        i.eval_str(b"interp alias $c add {} padd 100");
        assert_eq!(i.eval_str(b"$c eval {add 5}"), Code::Ok);
        assert_eq!(i.result_bytes(), b"105");
        // `$c alias name target prefix...`
        i.eval_str(b"$c alias mul ::tcl::mathop::* 3");
        assert_eq!(i.eval_str(b"$c eval {mul 4}"), Code::Ok);
        assert_eq!(i.result_bytes(), b"12");
        // Re-entrancy: a parent alias target that evals back into the same
        // child while its outer eval is still on the stack. This recurses
        // (the child's `x` ends up set), it does not error.
        i.eval_str(b"proc reenter {} { $::c eval {set x 42} }");
        i.eval_str(b"interp alias $c cb {} reenter");
        assert_eq!(i.eval_str(b"$c eval {cb}"), Code::Ok);
        assert_eq!(i.result_bytes(), b"42");
        assert_eq!(i.eval_str(b"$c eval {set x}"), Code::Ok);
        assert_eq!(i.result_bytes(), b"42");
        i.eval_str(b"interp delete $c; unset -nocomplain c");
    });
}

// Needs the numeric tower: the `-safe` hidden-list assert compares via `expr`.
#[cfg(have_tommath)]
#[test]
fn hidden_commands_and_safe() {
    // `interp hide`/`expose`/`invokehidden` + `interp create -safe`.
    leak_free(|i| {
        i.eval_str(b"set c [interp create]");
        i.eval_str(b"$c hide set");
        // hidden `set` is gone from the child but invocable via invokehidden.
        assert_eq!(i.eval_str(b"$c eval {set x 1}"), Code::Error);
        assert_eq!(i.eval_str(b"$c hidden"), Code::Ok);
        assert_eq!(i.result_bytes(), b"set");
        assert_eq!(i.eval_str(b"$c invokehidden set y 5"), Code::Ok);
        assert_eq!(i.result_bytes(), b"5");
        i.eval_str(b"$c expose set");
        assert_eq!(i.eval_str(b"$c eval {set z 9}"), Code::Ok);
        assert_eq!(i.result_bytes(), b"9");
        // `-safe` hides the host-touching commands it has.
        i.eval_str(b"set s [interp create -safe]");
        assert_eq!(
            i.eval_str(b"expr {[lsearch [$s hidden] file] >= 0}"),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"1");
        i.eval_str(b"interp delete $c; interp delete $s");
    });
}

#[test]
fn rename_moves_a_command() {
    leak_free(|i| {
        assert_eq!(i.eval_str(b"rename set put"), Code::Ok);
        assert_eq!(i.result_bytes(), b"");
        // `put` now works …
        assert_eq!(i.eval_str(b"put x 5"), Code::Ok);
        assert_eq!(i.result_bytes(), b"5");
        // … and `set` no longer resolves.
        assert_eq!(i.eval_str(b"set y 1"), Code::Error);
        assert_eq!(i.result_bytes(), b"invalid command name \"set\"");
    });
}

#[test]
fn rename_to_empty_deletes() {
    leak_free(|i| {
        assert_eq!(i.eval_str(b"rename incr {}"), Code::Ok);
        assert_eq!(i.eval_str(b"incr n"), Code::Error);
        assert_eq!(i.result_bytes(), b"invalid command name \"incr\"");
    });
}

#[test]
fn rename_missing_is_an_error() {
    leak_free(|i| {
        assert_eq!(i.eval_str(b"rename nope gone"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"can't rename \"nope\": command doesn't exist"
        );
        assert_eq!(i.eval_str(b"rename nope {}"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"can't delete \"nope\": command doesn't exist"
        );
    });
}

#[test]
fn rename_and_hide_error_shapes_and_lifecycle() {
    leak_free(|i| {
        // The verb is selected by the empty destination, including for a
        // qualified missing name (TclRenameCommand / tclNamesp.c).
        assert_eq!(i.eval_str(b"rename ::missing {}"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"can't delete \"::missing\": command doesn't exist"
        );

        // Qualified moves preserve the command and resolve from the new
        // namespace; deletion uses the same canonical rename seam.
        assert_eq!(
                i.eval_str(
                    b"namespace eval ::rename_ns {proc p {} {return qualified}}; rename ::rename_ns::p ::rename_ns::q"
                ),
                Code::Ok
            );
        assert_eq!(i.eval_str(b"::rename_ns::q"), Code::Ok);
        assert_eq!(i.result_bytes(), b"qualified");
        assert_eq!(i.eval_str(b"rename ::rename_ns::q {}"), Code::Ok);
        assert_eq!(i.eval_str(b"::rename_ns::q"), Code::Error);
        assert_eq!(i.result_bytes(), b"invalid command name \"::rename_ns::q\"");

        // Hiding an absent command is an error, so repeated hide cannot
        // silently swallow a typo in a security-sensitive setup.
        assert_eq!(i.eval_str(b"interp hide {} nosuchcmd"), Code::Error);
        assert_eq!(i.result_bytes(), b"unknown command \"nosuchcmd\"");
        assert_eq!(
            i.eval_str(b"proc visible {} {return visible}; interp hide {} visible"),
            Code::Ok
        );
        assert_eq!(i.eval_str(b"interp hidden {}"), Code::Ok);
        assert_eq!(i.result_bytes(), b"visible");
        // Hiding again once the source has moved into the hidden table is a
        // *lookup* miss, not a token collision: `Tcl_HideCommand` resolves
        // the source before it consults the hidden table, and the source is
        // no longer there to resolve (tclsh 8.6.16 / 9.0.4-pinned). Reaching
        // the collision needs a live source *and* a taken token, so the
        // command has to be re-created first.
        assert_eq!(i.eval_str(b"interp hide {} visible"), Code::Error);
        assert_eq!(i.result_bytes(), b"unknown command \"visible\"");
        assert_eq!(i.eval_str(b"proc visible {} {return shadow}"), Code::Ok);
        assert_eq!(i.eval_str(b"interp hide {} visible"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"hidden command named \"visible\" already exists"
        );
        assert_eq!(i.eval_str(b"rename visible {}"), Code::Ok);
        assert_eq!(i.eval_str(b"interp invokehidden {} visible"), Code::Ok);
        assert_eq!(i.result_bytes(), b"visible");
        assert_eq!(i.eval_str(b"interp expose {} visible"), Code::Ok);
        assert_eq!(i.eval_str(b"visible"), Code::Ok);
        assert_eq!(i.result_bytes(), b"visible");

        // Destination conflicts are diagnosed before either table is
        // mutated. Both the source command and existing destination keep
        // their identities and remain callable.
        assert_eq!(
            i.eval_str(
                b"proc p {} {return P}
                      interp hide {} p held
                      proc q {} {return Q}
                      set c [catch {interp hide {} q held} m o]
                      list $c $m [dict get $o -errorcode] [q] \
                           [interp invokehidden {} held] [interp hidden {}]"
            ),
            Code::Ok
        );
        assert_eq!(
            i.result_bytes(),
            b"1 {hidden command named \"held\" already exists} \
                  {TCL HIDE ALREADY_HIDDEN} Q P held"
        );

        assert_eq!(
            i.eval_str(
                b"proc E {} {return E}
                      set c [catch {interp expose {} held E} m o]
                      list $c $m [dict get $o -errorcode] [E] \
                           [interp invokehidden {} held] [interp hidden {}]"
            ),
            Code::Ok
        );
        assert_eq!(
            i.result_bytes(),
            b"1 {exposed command \"E\" already exists} \
                  {TCL EXPOSE COMMAND_EXISTS} E P held"
        );

        // Release-hidden builtins are absent from both rename and hide;
        // neither operation may make an 8.5+ command callable in 8.4.
        i.set_runtime_version(tcl_dialect::TclVersion::V8_4);
        assert_eq!(i.eval_str(b"rename lassign {}"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"can't delete \"lassign\": command doesn't exist"
        );
        assert_eq!(i.eval_str(b"interp hide {} lassign"), Code::Error);
        assert_eq!(i.result_bytes(), b"unknown command \"lassign\"");
    });
}

/// The `$child` shorthand now routes through `hidectl_in` /
/// `invokehidden_in`, and the latter moved the manual
/// `incr_ref_count`/`decr_ref_count` bracketing around the hidden argv out
/// of two call sites into one. The counter harness is the guard on that
/// move: a dropped `+1` or a missing `-1` shows up here and nowhere else.
#[test]
fn child_shorthand_visibility_ops_are_leak_free() {
    leak_free(|i| {
        assert_eq!(i.eval_str(b"set c [interp create]"), Code::Ok);
        assert_eq!(i.eval_str(b"$c eval {proc lst {} {return L}}"), Code::Ok);
        assert_eq!(i.eval_str(b"$c hide lst mylst"), Code::Ok);
        assert_eq!(i.eval_str(b"$c hidden"), Code::Ok);
        assert_eq!(i.result_bytes(), b"mylst");
        assert_eq!(i.eval_str(b"$c invokehidden mylst"), Code::Ok);
        assert_eq!(i.result_bytes(), b"L");
        assert_eq!(i.eval_str(b"$c expose mylst lst2"), Code::Ok);
        assert_eq!(i.eval_str(b"$c hide set"), Code::Ok);
        assert_eq!(i.eval_str(b"$c invokehidden -global set g 1"), Code::Ok);
        assert_eq!(i.result_bytes(), b"1");
        assert_eq!(
            i.eval_str(b"$c invokehidden -namespace foo set q 2"),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"2");
        assert_eq!(
            i.eval_str(b"catch {$c invokehidden -bogus set x 1} m; set m"),
            Code::Ok
        );
        assert_eq!(
            i.result_bytes(),
            b"bad option \"-bogus\": must be -global, -namespace, or --"
        );
        assert_eq!(i.eval_str(b"catch {$c hide ::foo::bar} m; set m"), Code::Ok);
        assert_eq!(
            i.result_bytes(),
            b"cannot use namespace qualifiers in hidden command token (rename)"
        );
        assert_eq!(i.eval_str(b"interp delete $c"), Code::Ok);
    });
}

#[test]
fn visibility_failures_match_direct_and_child_oracles() {
    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"set out {}
                      foreach script {
                          {interp hide {} nosuch}
                          {interp expose {} nosuch}
                      } {
                          catch $script m o
                          lappend out $m [dict get $o -errorcode]
                      }
                      proc held {} {return OLD}
                      interp hide {} held
                      proc held {} {return NEW}
                      catch {interp hide {} held} m o
                      lappend out $m [dict get $o -errorcode]
                      proc E {} {return OLD}
                      interp hide {} E
                      proc E {} {return NEW}
                      catch {interp expose {} E} m o
                      lappend out $m [dict get $o -errorcode]
                      set out"
            ),
            Code::Ok
        );
        assert_eq!(
            i.result_bytes(),
            b"{unknown command \"nosuch\"} {TCL LOOKUP COMMAND nosuch} \
                  {unknown hidden command \"nosuch\"} \
                  {TCL LOOKUP HIDDENTOKEN nosuch} \
                  {hidden command named \"held\" already exists} \
                  {TCL HIDE ALREADY_HIDDEN} \
                  {exposed command \"E\" already exists} \
                  {TCL EXPOSE COMMAND_EXISTS}"
        );
    });

    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"set c [interp create]
                      set out {}
                      foreach subcommand {hide expose} {
                          catch [list $c $subcommand nosuch] m o
                          lappend out $m [dict get $o -errorcode]
                      }
                      $c eval {proc held {} {return OLD}}
                      $c hide held
                      $c eval {proc held {} {return NEW}}
                      catch [list $c hide held] m o
                      lappend out $m [dict get $o -errorcode]
                      $c eval {proc E {} {return OLD}}
                      $c hide E
                      $c eval {proc E {} {return NEW}}
                      catch [list $c expose E] m o
                      lappend out $m [dict get $o -errorcode]
                      set out"
            ),
            Code::Ok
        );
        assert_eq!(
            i.result_bytes(),
            b"{unknown command \"nosuch\"} {TCL LOOKUP COMMAND nosuch} \
                  {unknown hidden command \"nosuch\"} \
                  {TCL LOOKUP HIDDENTOKEN nosuch} \
                  {hidden command named \"held\" already exists} \
                  {TCL HIDE ALREADY_HIDDEN} \
                  {exposed command \"E\" already exists} \
                  {TCL EXPOSE COMMAND_EXISTS}"
        );
    });
}

/// C Tcl has no protected-command list for `rename`: even `return` may
/// be renamed and used under its new name (tclsh 8.6.16 / 9.0.4:
/// `rename ::return ::myreturn` succeeds).
#[test]
fn rename_builtin_return_is_allowed() {
    leak_free(|i| {
        assert_eq!(i.eval_str(b"rename return myreturn"), Code::Ok);
        assert_eq!(i.eval_str(b"proc p {} { myreturn done }"), Code::Ok);
        assert_eq!(i.eval_str(b"p"), Code::Ok);
        assert_eq!(i.result_bytes(), b"done");
        // Restore for any later tests in this interp.
        assert_eq!(i.eval_str(b"rename myreturn return"), Code::Ok);
    });
}

#[test]
fn alias_create_and_dispatch() {
    leak_free(|i| {
        // `=` is an alias for `set`.
        assert_eq!(i.eval_str(b"interp alias {} = {} set"), Code::Ok);
        assert_eq!(i.result_bytes(), b"=");
        assert_eq!(i.eval_str(b"= x 42"), Code::Ok);
        assert_eq!(i.result_bytes(), b"42");
        assert_eq!(i.eval_str(b"set y $x"), Code::Ok);
        assert_eq!(i.result_bytes(), b"42");
    });
}

#[test]
fn alias_prepends_frozen_prefix() {
    leak_free(|i| {
        // `store` aliases `set k`, so `store v` runs `set k v`.
        assert_eq!(i.eval_str(b"interp alias {} store {} set k"), Code::Ok);
        assert_eq!(i.eval_str(b"store hello"), Code::Ok);
        assert_eq!(i.result_bytes(), b"hello");
        assert_eq!(i.eval_str(b"set out $k"), Code::Ok);
        assert_eq!(i.result_bytes(), b"hello");
    });
}

#[test]
fn alias_query_returns_target_and_prefix() {
    leak_free(|i| {
        i.eval_str(b"interp alias {} store {} set k");
        assert_eq!(i.eval_str(b"interp alias {} store"), Code::Ok);
        assert_eq!(i.result_bytes(), b"set k");
    });
}

#[test]
fn alias_delete_unbinds() {
    leak_free(|i| {
        i.eval_str(b"interp alias {} = {} set");
        assert_eq!(i.eval_str(b"interp alias {} = {}"), Code::Ok);
        assert_eq!(i.eval_str(b"= x 1"), Code::Error);
        assert_eq!(i.result_bytes(), b"invalid command name \"=\"");
    });
}

#[test]
fn aliases_lists_every_alias() {
    leak_free(|i| {
        i.eval_str(b"interp alias {} = {} set");
        i.eval_str(b"interp alias {} store {} set k");
        assert_eq!(i.eval_str(b"interp aliases {}"), Code::Ok);
        // Exact enumeration for this fixture on the pinned C Tcl
        // 8.4 through 9.1 providers is `store =`.
        assert_eq!(i.result_bytes(), b"store =");
    });
}

#[test]
fn alias_to_deleted_target_errors_lazily() {
    leak_free(|i| {
        i.eval_str(b"interp alias {} = {} set");
        // Deleting the target makes the alias fail at *its* next dispatch,
        // attributing the miss to the target name.
        assert_eq!(i.eval_str(b"rename set {}"), Code::Ok);
        assert_eq!(i.eval_str(b"= x 1"), Code::Error);
        assert_eq!(i.result_bytes(), b"invalid command name \"set\"");
    });
}

#[test]
fn alias_does_not_follow_target_rename() {
    leak_free(|i| {
        i.eval_str(b"interp alias {} = {} set");
        // Renaming the target out from under the alias: the stored name stops
        // resolving (C Tcl semantics — alias binds by name, not by command).
        assert_eq!(i.eval_str(b"rename set put"), Code::Ok);
        assert_eq!(i.eval_str(b"= x 1"), Code::Error);
        assert_eq!(i.result_bytes(), b"invalid command name \"set\"");
    });
}

/// C's `TclPreventAliasLoop` (`tclInterp.c`) refuses the alias that closes
/// a cycle at *definition* time — the pair installs fine in neither order
/// (tclsh 8.6.16 / 9.0.4-pinned wording, shared with the VM's
/// `cross_interp_alias_e2e` vectors).
#[test]
fn a_mutual_alias_pair_is_refused_at_the_closing_alias() {
    leak_free(|i| {
        assert_eq!(i.eval_str(b"interp alias {} a {} b"), Code::Ok);
        assert_eq!(i.eval_str(b"interp alias {} b {} a"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"cannot define or rename alias \"b\": would create a loop"
        );
        // The refused alias is not left behind …
        assert_eq!(i.eval_str(b"info commands b"), Code::Ok);
        assert_eq!(i.result_bytes(), b"");
        // … so the surviving one still just misses its (absent) target
        // instead of recursing without bound.
        assert_eq!(i.eval_str(b"a"), Code::Error);
        assert_eq!(i.result_bytes(), b"invalid command name \"b\"");
    });
}

/// A self-alias is refused too — and, as in C, the command it displaced is
/// *not* restored (the alias is created first, then unbound on the loop).
#[test]
fn a_self_alias_is_refused_and_destroys_the_command_it_displaced() {
    leak_free(|i| {
        assert_eq!(i.eval_str(b"proc x {} {return REAL}"), Code::Ok);
        assert_eq!(i.eval_str(b"interp alias {} x {} x"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"cannot define or rename alias \"x\": would create a loop"
        );
        assert_eq!(i.eval_str(b"x"), Code::Error);
        assert_eq!(i.result_bytes(), b"invalid command name \"x\"");
        // A qualified self-spelling is caught by *resolution*, not by a
        // string compare of the two names.
        assert_eq!(i.eval_str(b"interp alias {} q {} ::q"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"cannot define or rename alias \"q\": would create a loop"
        );
    });
}

/// The walk follows the whole chain, not just one hop, and a frozen prefix
/// on the closing alias does not hide the cycle.
#[test]
fn a_longer_alias_cycle_is_refused_at_its_closing_hop() {
    leak_free(|i| {
        assert_eq!(i.eval_str(b"interp alias {} c {} d"), Code::Ok);
        assert_eq!(i.eval_str(b"interp alias {} d {} e"), Code::Ok);
        assert_eq!(i.eval_str(b"interp alias {} e {} c extra"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"cannot define or rename alias \"e\": would create a loop"
        );
        // A namespaced cycle is refused the same way, named by the alias's
        // simple command name (C's `Tcl_GetCommandName`), not its path.
        assert_eq!(i.eval_str(b"namespace eval ns {}"), Code::Ok);
        assert_eq!(i.eval_str(b"interp alias {} ns::p {} ns::q"), Code::Ok);
        assert_eq!(i.eval_str(b"interp alias {} ns::q {} ns::p"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"cannot define or rename alias \"q\": would create a loop"
        );
    });
}

/// C guards `rename` with the same check: moving an alias onto a name its
/// own target chain resolves back to is refused, and the table is left
/// exactly as it was.
#[test]
fn a_rename_that_would_close_a_loop_is_refused_and_rolled_back() {
    leak_free(|i| {
        assert_eq!(i.eval_str(b"interp alias {} a {} b"), Code::Ok);
        assert_eq!(i.eval_str(b"rename a b"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"cannot define or rename alias \"b\": would create a loop"
        );
        assert_eq!(i.eval_str(b"info commands a"), Code::Ok);
        assert_eq!(i.result_bytes(), b"a");
        assert_eq!(i.eval_str(b"info commands b"), Code::Ok);
        assert_eq!(i.result_bytes(), b"");
    });
}

/// The refused rename must not damage the command sitting at the
/// destination either: the tentative move the gate makes is undone
/// completely. (tclsh refuses this one earlier still, with `can't rename
/// to "bb": command already exists` — this runtime's `rename` has no
/// destination-exists check, so it reports the loop instead; the surviving
/// table state is the same either way.)
#[test]
fn a_refused_rename_leaves_the_destination_command_intact() {
    leak_free(|i| {
        assert_eq!(i.eval_str(b"proc bb {} {return REAL}"), Code::Ok);
        assert_eq!(i.eval_str(b"interp alias {} aa {} bb"), Code::Ok);
        assert_eq!(i.eval_str(b"rename aa bb"), Code::Error);
        assert_eq!(i.eval_str(b"bb"), Code::Ok);
        assert_eq!(i.result_bytes(), b"REAL");
        assert_eq!(i.eval_str(b"aa"), Code::Ok);
        assert_eq!(i.result_bytes(), b"REAL");
    });
}

/// The gate refuses cycles only: a legitimate alias-of-alias chain still
/// dispatches, and an alias to a target that does not exist yet is legal
/// (aliases late-bind).
#[test]
fn legitimate_alias_chains_and_late_binding_still_work() {
    leak_free(|i| {
        assert_eq!(i.eval_str(b"interp alias {} = {} set"), Code::Ok);
        assert_eq!(i.eval_str(b"interp alias {} := {} ="), Code::Ok);
        assert_eq!(i.eval_str(b":= zz 42"), Code::Ok);
        assert_eq!(i.result_bytes(), b"42");
        assert_eq!(i.eval_str(b"set out $zz"), Code::Ok);
        assert_eq!(i.result_bytes(), b"42");
        assert_eq!(i.eval_str(b"interp alias {} lb {} no_such_yet"), Code::Ok);
        assert_eq!(i.eval_str(b"proc no_such_yet {} {return LATE}"), Code::Ok);
        assert_eq!(i.eval_str(b"lb"), Code::Ok);
        assert_eq!(i.result_bytes(), b"LATE");
        // Renaming a non-alias, and renaming an alias somewhere harmless,
        // are both unaffected by the gate.
        assert_eq!(i.eval_str(b"rename := assign"), Code::Ok);
        assert_eq!(i.eval_str(b"assign zz 7"), Code::Ok);
        assert_eq!(i.result_bytes(), b"7");
    });
}

/// Definition-direction parity: a written trailing separator names
/// the empty-string `{}` command inside its full qualifier chain — for
/// `proc`, `rename`'s NEW name, and dispatch alike (`proc x:: {} {…}`
/// defines `::x::` and `x::` invokes it; `rename foo x::` / `rename bar
/// ::` rebind the `{}` command — all tclsh 8.6.16/9.0.4-pinned). The
/// definition split must keep the empty tail, or the proc just defined
/// could not be invoked.
#[test]
fn trailing_separator_definitions_match_resolution() {
    leak_free(|i| {
        // proc with a trailing separator: defined AND callable.
        assert_eq!(
            i.eval_str(b"namespace eval x {}; proc x:: {} { return EMPTYTAIL }"),
            Code::Ok
        );
        assert_eq!(i.eval_str(b"x::"), Code::Ok);
        assert_eq!(i.result_bytes(), b"EMPTYTAIL");
        // … and reachable via any separator-run spelling.
        assert_eq!(i.eval_str(b"::x::"), Code::Ok);
        assert_eq!(i.result_bytes(), b"EMPTYTAIL");
        // rename TO a trailing-separator name binds the `{}` command.
        assert_eq!(
            i.eval_str(b"proc foo {} { return F }; rename foo y::"),
            Code::Ok
        );
        assert_eq!(i.eval_str(b"y::"), Code::Ok);
        assert_eq!(i.result_bytes(), b"F");
        // rename to `::` binds the GLOBAL `{}` command.
        assert_eq!(
            i.eval_str(b"proc bar {} { return B }; rename bar ::"),
            Code::Ok
        );
        assert_eq!(i.eval_str(b"::"), Code::Ok);
        assert_eq!(i.result_bytes(), b"B");
        // The `{}` command can be renamed AWAY from its spelling too.
        assert_eq!(i.eval_str(b"rename y:: plain"), Code::Ok);
        assert_eq!(i.eval_str(b"plain"), Code::Ok);
        assert_eq!(i.result_bytes(), b"F");
        assert_eq!(i.eval_str(b"y::"), Code::Error);
        // Variables share the rule: `set vx:: V` writes the `{}` variable
        // in `::vx` (tclsh: `info vars ::vx::*` lists `::vx::`).
        assert_eq!(i.eval_str(b"namespace eval vx {}; set vx:: V"), Code::Ok);
        assert_eq!(i.eval_str(b"set vx::"), Code::Ok);
        assert_eq!(i.result_bytes(), b"V");
        // And ensembles: `namespace ensemble create -command ::e::` binds
        // the `{}` command in `::e` (tclsh-pinned: `e:: go` dispatches).
        assert_eq!(
            i.eval_str(
                b"namespace eval e { proc go {} {return GO}; namespace export go; \
                       namespace ensemble create -command ::e:: }"
            ),
            Code::Ok
        );
        assert_eq!(i.eval_str(b"e:: go"), Code::Ok);
        assert_eq!(i.result_bytes(), b"GO");
        // A proc named `:` inside a namespace named `:` has NO absolute
        // spelling (W314's case) but IS reachable by relative dispatch
        // from inside its namespace — `namespace eval : { : }` and
        // `namespace inscope : :` both invoke it, while every absolute
        // spelling misses (tclsh 8.6.16/9.0.4-pinned).
        assert_eq!(
            i.eval_str(b"namespace eval : { proc : args { return hello } }"),
            Code::Ok
        );
        assert_eq!(i.eval_str(b"namespace eval : { : }"), Code::Ok);
        assert_eq!(i.result_bytes(), b"hello");
        assert_eq!(i.eval_str(b"namespace inscope : :"), Code::Ok);
        assert_eq!(i.result_bytes(), b"hello");
        // Every all-colon spelling is separator runs around an empty
        // tail — it can only ever reach the GLOBAL `{}` command (bound
        // above by `rename bar ::`), never the `:`-named proc
        // (tclsh-pinned: prints B here, and errors once no global `{}`
        // exists).
        assert_eq!(i.eval_str(b":::::"), Code::Ok);
        assert_eq!(i.result_bytes(), b"B");
    });
}
