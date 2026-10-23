// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual original namespace-result objects compared with captured C producers.

use super::*;
use crate::{counters, interp::Code};
use tcl_syntax::native_object::NativeObjectCacheSnapshot as Snapshot;

const OBSERVATIONS: &str = include_str!(
    "../../../../../rust/tcl-syntax/tests/data/native_namespace_name/producer_getter.txt"
);

fn primary(value: *mut TclObj) -> (&'static str, bool) {
    match obj::native_object_snapshot(value).unwrap().cache {
        Snapshot::None => ("none", false),
        Snapshot::String { .. } | Snapshot::JimString { .. } => ("string", false),
        Snapshot::ByteArray { .. } => ("bytearray", false),
        Snapshot::NamespaceName { resolved, .. } => ("nsName", resolved),
        other => panic!("unexpected namespace result primary: {other:?}"),
    }
}

fn check(
    interp: &mut Interp,
    version: &str,
    phase: &str,
    value: *mut TclObj,
    code: Code,
    input: Option<*mut TclObj>,
) {
    let fields: Vec<_> = OBSERVATIONS
        .lines()
        .find(|row| row.starts_with(&format!("{version}|{phase}|")))
        .expect("native result window")
        .split('|')
        .collect();
    assert_eq!(
        code.as_int(),
        fields[2].parse::<i64>().unwrap(),
        "{version} {phase}: {:?}; refusal={:?}",
        interp.result_bytes(),
        interp.native_access_refusal()
    );
    assert_eq!(
        primary(value).0,
        fields[3],
        "{version} {phase} original result primary"
    );
    let bytes = interp.native_string_bytes(&value).unwrap();
    let expected: Vec<_> = fields[4]
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect();
    assert_eq!(
        bytes.as_ref(),
        expected,
        "{version} {phase} counted result bytes"
    );
    if let Some(input) = input {
        assert_eq!(
            primary(input),
            (fields[5], fields[6] == "true"),
            "{version} {phase} original input getter effect"
        );
    }
}

fn invoke(interp: &mut Interp, member: &[u8], input: Option<*mut TclObj>) -> Code {
    let command = obj::Owned::fresh(obj::new_string_bytes(b"namespace"));
    let member = obj::Owned::fresh(obj::new_string_bytes(member));
    let mut args = vec![command.as_ptr(), member.as_ptr()];
    if let Some(input) = input {
        args.push(input);
    }
    interp.dispatch(&args)
}

fn push_namespace(interp: &mut Interp, namespace: NsId) {
    interp.current_ns.set(namespace);
    interp.enter_namespace_activation(namespace);
    interp.frames.borrow_mut().push_namespace(namespace);
}

fn parent(interp: &mut Interp, version: &str, phase: &str, original: *mut TclObj) {
    let code = invoke(interp, b"parent", Some(original));
    let result = interp.result_obj();
    check(interp, version, phase, result, code, Some(original));
}

#[test]
fn opaque_children_match_20_native_query_and_primary_windows() {
    // Native proof: naming.namespace-native.opaque-child-enumeration

    // Native proof: naming.namespace-native.opaque-child-pattern-selection

    let observations = include_str!(
        "../../../../../rust/tcl-syntax/tests/data/native_namespace_name/opaque_children.txt"
    );
    let mut matched = 0;
    for (version, native) in [
        (tcl_dialect::TclVersion::V8_4, "8.4.20"),
        (tcl_dialect::TclVersion::V8_5, "8.5.19"),
        (tcl_dialect::TclVersion::V8_6, "8.6.18"),
        (tcl_dialect::TclVersion::V9_0, "9.0.4"),
        (tcl_dialect::TclVersion::V9_1, "9.1.0"),
    ] {
        let mut interp = Interp::new();
        interp.set_runtime_version(version);
        interp.ensure_namespace(b"::raw\xff");
        interp.ensure_namespace(b"::raw\xff::child\xfd");
        for (phase, pattern) in [
            ("enumerate", None),
            ("exact", Some(&b"::raw\xff::child\xfd"[..])),
            ("star", Some(&b"*"[..])),
            ("negative", Some(&b"*other*"[..])),
        ] {
            let expected: Vec<_> = observations
                .lines()
                .find(|row| row.starts_with(&format!("{native}|{phase}|")))
                .unwrap()
                .split('|')
                .collect();
            let mut words = vec![
                obj::Owned::fresh(obj::new_string_bytes(b"namespace")),
                obj::Owned::fresh(obj::new_string_bytes(b"children")),
                obj::Owned::fresh(obj::new_string_bytes(b"::raw\xff")),
            ];
            if let Some(pattern) = pattern {
                words.push(obj::Owned::fresh(obj::new_string_bytes(pattern)));
            }
            let argv: Vec<_> = words.iter().map(obj::Owned::as_ptr).collect();
            assert_eq!(interp.dispatch(&argv), Code::Ok, "{native} {phase}");
            let result = interp.result_obj();
            let primary = match obj::native_object_snapshot(result).unwrap().cache {
                Snapshot::None => "none",
                Snapshot::List { .. } => "list",
                other => panic!("unexpected original child result: {other:?}"),
            };
            assert_eq!(primary, expected[3], "{native} {phase}");
            let bytes = interp.native_string_bytes(&result).unwrap();
            let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
            assert_eq!(hex, expected[4], "{native} {phase}");
            matched += 1;
        }
    }
    assert_eq!(matched, 20);
}

#[test]
fn original_namespace_objects_match_100_native_c_producer_and_getter_windows() {
    // Native proof: naming.namespace-native.current-root-primary

    // Native proof: naming.namespace-native.terminal-colon-original-versus-text

    // Native proof: naming.namespace-native.deleted-current-object-recreation

    // Native proof: naming.namespace-native.raw-zero-bytearray-parent

    // Native proof: naming.namespace-native.opaque-original-parent

    // Native proof: naming.namespace-native.active-retirement-cache

    // Native proof: naming.namespace-native.missing-parent-cache

    for (version, release) in [
        (tcl_dialect::TclVersion::V8_4, "8.4.20"),
        (tcl_dialect::TclVersion::V8_5, "8.5.19"),
        (tcl_dialect::TclVersion::V8_6, "8.6.18"),
        (tcl_dialect::TclVersion::V9_0, "9.0.4"),
        (tcl_dialect::TclVersion::V9_1, "9.1.0"),
    ] {
        counters::reset();
        {
            let mut interp = Interp::new();
            interp.set_runtime_version(version);
            interp.ensure_namespace(b"::a");
            let code = invoke(&mut interp, b"current", None);
            let result = interp.result_obj();
            check(&mut interp, release, "root-current", result, code, None);
            let root = obj::Owned::retain(interp.result_obj());
            let duplicate = obj::Owned::fresh(obj::duplicate(root.as_ptr()));
            check(
                &mut interp,
                release,
                "root-duplicate",
                duplicate.as_ptr(),
                Code::Ok,
                None,
            );
            parent(&mut interp, release, "root-parent", duplicate.as_ptr());
            drop(duplicate);
            drop(root);
            let outer = interp.ensure_namespace(b"::a:");
            push_namespace(&mut interp, outer);
            let child = interp.ensure_namespace(b"q");
            interp.codegen_frame_pop();
            push_namespace(&mut interp, child);
            let code = invoke(&mut interp, b"current", None);
            let result = interp.result_obj();
            check(
                &mut interp,
                release,
                "terminal-colon-child-current",
                result,
                code,
                None,
            );
            let original = obj::Owned::retain(interp.result_obj());
            let duplicate = obj::Owned::fresh(obj::duplicate(original.as_ptr()));
            check(
                &mut interp,
                release,
                "nested-duplicate",
                duplicate.as_ptr(),
                Code::Ok,
                None,
            );
            if version >= tcl_dialect::TclVersion::V9_0 {
                assert!(obj::native_namespace_name::cache(original.as_ptr())
                    .unwrap()
                    .same_descriptor(
                        &obj::native_namespace_name::cache(duplicate.as_ptr()).unwrap()
                    ));
            }
            interp.codegen_frame_pop();
            parent(
                &mut interp,
                release,
                "current-object-parent-from-root",
                original.as_ptr(),
            );
            let written = obj::Owned::fresh(obj::new_string_bytes(b"::a:::q"));
            parent(
                &mut interp,
                release,
                "reported-text-parent-from-root",
                written.as_ptr(),
            );
            interp.delete_namespace_by_id(child);
            parent(
                &mut interp,
                release,
                "deleted-current-object-parent",
                original.as_ptr(),
            );
            parent(
                &mut interp,
                release,
                "deleted-duplicate-parent",
                duplicate.as_ptr(),
            );
            push_namespace(&mut interp, outer);
            let recreated = interp.ensure_namespace(b"q");
            assert_ne!(child, recreated);
            interp.codegen_frame_pop();
            parent(
                &mut interp,
                release,
                "recreated-current-object-parent",
                original.as_ptr(),
            );
            let nul = obj::Owned::fresh(obj::new_string_bytes(b"::a\0z"));
            parent(&mut interp, release, "raw-NUL-parent", nul.as_ptr());
            let modified = obj::Owned::fresh(obj::new_string_bytes(b"::a\xc0\x80z"));
            parent(
                &mut interp,
                release,
                "modified-NUL-negative-parent",
                modified.as_ptr(),
            );
            let binary = obj::Owned::fresh(interp.new_native_byte_array(b"::a\0z").unwrap());
            parent(
                &mut interp,
                release,
                "pure-ByteArray-negative-parent",
                binary.as_ptr(),
            );
            interp.ensure_namespace(b"::a\xc0\x80z");
            parent(
                &mut interp,
                release,
                "modified-NUL-positive-parent",
                modified.as_ptr(),
            );
            parent(
                &mut interp,
                release,
                "pure-ByteArray-positive-parent",
                binary.as_ptr(),
            );
            interp.ensure_namespace(b"::\xff");
            let opaque = obj::Owned::fresh(obj::new_string_bytes(b"::\xff"));
            parent(
                &mut interp,
                release,
                "raw-FF-positive-parent",
                opaque.as_ptr(),
            );
            let retiring = interp.ensure_namespace(b"::Z");
            let name = obj::Owned::fresh(obj::new_string_bytes(b"::Z"));
            parent(&mut interp, release, "active-retire-before", name.as_ptr());
            push_namespace(&mut interp, retiring);
            interp.delete_namespace_by_id(retiring);
            parent(&mut interp, release, "active-retire-cached", name.as_ptr());
            interp.codegen_frame_pop();
            parent(
                &mut interp,
                release,
                "active-retire-after-pop",
                name.as_ptr(),
            );
            let missing = obj::Owned::fresh(obj::new_string_bytes(b"::missing"));
            parent(&mut interp, release, "fresh-missing", missing.as_ptr());
        }
        assert_eq!(
            counters::finalize(),
            0,
            "{release} leaked namespace object leases"
        );
        assert_eq!(counters::double_free_count(), 0);
    }
}

#[test]
fn native_namespace_cache_cannot_authenticate_a_same_number_replacement() {
    let mut interp = Interp::new();
    let parent = interp.ensure_namespace(b"::a:");
    push_namespace(&mut interp, parent);
    let child = interp.ensure_namespace(b"q");
    interp.codegen_frame_pop();
    let original = obj::Owned::fresh(obj::new_string_bytes(b"::a:::q"));
    let counterfeit = tcl_runtime_api::native_namespace_name::NativeNamespaceNameToken::new(
        interp.native_command_interpreter,
        child as u64,
        tcl_core_types::NameBytes::from("::a:::q"),
        Some(parent as u64),
    );
    let protocol = interp
        .native_invocation_dialect()
        .native_namespace_name_protocol()
        .unwrap();
    obj::native_namespace_name::install(
        original.as_ptr(),
        Cache::resolved(protocol.recipe().version(), counterfeit, None),
        protocol,
    )
    .unwrap();
    assert_eq!(
        interp
            .native_namespace_object_lookup(original.as_ptr())
            .unwrap(),
        None
    );
    assert!(obj::native_namespace_name::cache(original.as_ptr()).is_none());
}

#[test]
fn current_namespace_results_match_current_jim_original_producers() {
    // Native proof: naming.namespace-native.current-root-primary

    // Native proof: naming.namespace-native.jim-nested-current-primary

    let profile = Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
        "jim",
        &[],
        "Jim",
        tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
    )));
    let mut interp = Interp::new();
    interp.set_dialect_profile(profile);
    let code = invoke(&mut interp, b"current", None);
    let result = interp.result_obj();
    check(&mut interp, "Jim", "root-current", result, code, None);
    let duplicate = obj::Owned::fresh(obj::duplicate(result));
    check(
        &mut interp,
        "Jim",
        "root-duplicate",
        duplicate.as_ptr(),
        Code::Ok,
        None,
    );
    assert_ne!(
        interp.jim_current_namespace_object().unwrap().as_ptr(),
        result
    );
    let code = interp.eval_str(b"namespace eval ::N {namespace eval q {namespace current}}");
    let result = interp.result_obj();
    check(&mut interp, "Jim", "nested-current", result, code, None);
}

#[test]
fn original_namespace_lifetime_survives_retirement_without_owning_the_interpreter() {
    let original;
    let token;
    {
        let mut interp = Interp::new();
        interp.set_runtime_version(tcl_dialect::TclVersion::V8_4);
        let namespace = interp.ensure_namespace(b"::Z");
        original = obj::Owned::fresh(obj::new_string_bytes(b"::Z"));
        assert_eq!(
            interp
                .native_namespace_object_lookup(original.as_ptr())
                .unwrap(),
            Some(namespace)
        );
        token = obj::native_namespace_name::cache(original.as_ptr())
            .unwrap()
            .namespace()
            .unwrap()
            .clone();
        let duplicate = obj::Owned::fresh(obj::duplicate(original.as_ptr()));
        assert!(obj::native_namespace_name::cache(original.as_ptr())
            .unwrap()
            .same_descriptor(&obj::native_namespace_name::cache(duplicate.as_ptr()).unwrap()));
        obj::invalidate_string(original.as_ptr());
        assert_eq!(
            interp
                .native_string_bytes(&original.as_ptr())
                .unwrap()
                .as_ref(),
            b"::Z"
        );
    }
    assert_eq!(
        token.lifecycle(),
        tcl_syntax::native_namespace_name::NativeNamespaceLifecycle::Dead
    );
    obj::invalidate_string(original.as_ptr());
    let protocol =
        tcl_syntax::native_string::NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4);
    assert_eq!(
        crate::dict::native_object_bytes(original.as_ptr(), protocol).unwrap(),
        b""
    );
    assert!(obj::has_canonical_empty_string(original.as_ptr()));
}

#[test]
fn namespace_delete_validates_every_original_before_retirement_and_relooks_after_callbacks() {
    // Native proof: naming.namespace-native.delete-validates-later

    // Native proof: naming.namespace-native.delete-dependent-child-duplicate

    // Native proof: naming.namespace-native.delete-unset-trace-later-effects

    for version in tcl_dialect::TclVersion::ALL {
        let mut interp = Interp::new();
        interp.set_runtime_version(version);
        interp.ensure_namespace(b"::A");
        interp.ensure_namespace(b"::A::child");
        let a = obj::Owned::fresh(obj::new_string_bytes(b"::A"));
        let missing = obj::Owned::fresh(obj::new_string_bytes(b"::missing"));
        assert_eq!(
            tcl_cmd_core::namespace::delete_original(&mut interp, &[a.as_ptr(), missing.as_ptr()])
                .unwrap(),
            Some(1)
        );
        assert!(interp.find_namespace_id(b"::A").is_some());
        assert!(interp.find_namespace_id(b"::A::child").is_some());
        let child = obj::Owned::fresh(obj::new_string_bytes(b"::A::child"));
        assert_eq!(
            tcl_cmd_core::namespace::delete_original(
                &mut interp,
                &[a.as_ptr(), child.as_ptr(), a.as_ptr()]
            )
            .unwrap(),
            None
        );
        assert!(interp.find_namespace_id(b"::A").is_none());
        for recreate in [false, true] {
            let setup = if recreate {
                b"namespace eval ::A {}; namespace eval ::B {}; set ::calls 0; set ::A::x 1; proc dependent {a b c} {incr ::calls; namespace delete ::B; namespace eval ::B {set fresh 1}}".as_slice()
            } else {
                b"namespace eval ::A {}; namespace eval ::B {}; set ::calls 0; set ::A::x 1; proc dependent {a b c} {incr ::calls; namespace delete ::B}".as_slice()
            };
            assert_eq!(
                interp.eval_str(setup),
                Code::Ok,
                "{version:?}: {:?}",
                interp.result_bytes()
            );
            let trace = if version < tcl_dialect::TclVersion::V9_0 {
                b"trace variable ::A::x u dependent".as_slice()
            } else {
                b"trace add variable ::A::x unset dependent".as_slice()
            };
            assert_eq!(interp.eval_str(trace), Code::Ok);
            let b = obj::Owned::fresh(obj::new_string_bytes(b"::B"));
            let command = obj::Owned::fresh(obj::new_string_bytes(b"namespace"));
            let member = obj::Owned::fresh(obj::new_string_bytes(b"delete"));
            assert_eq!(
                interp.dispatch(&[command.as_ptr(), member.as_ptr(), a.as_ptr(), b.as_ptr()]),
                Code::Ok,
                "{version:?} recreate={recreate}: {:?}; refusal={:?}",
                interp.result_bytes(),
                interp.native_access_refusal()
            );
            assert!(interp.result_bytes().is_empty());
            assert!(interp.find_namespace_id(b"::A").is_none());
            assert!(interp.find_namespace_id(b"::B").is_none());
            assert_eq!(interp.read_var(b"::calls", None).unwrap().as_slice(), b"1");
        }
    }
}

fn compare_empty_namespace_address_rows(environment: &str, observation: &str) {
    let rows = tcl_syntax::list::split_list(observation).unwrap();
    assert_eq!(rows.len(), 90);
    let mut active_identity = String::new();
    let mut interpreter = None;
    let mut compared = 0;
    let mut outside_ascii_scope = Vec::new();
    for row in rows.chunks_exact(5) {
        let id = tcl_syntax::list::split_list(&row[0]).unwrap();
        let identity = tcl_syntax::list::split_list(&id[0]).unwrap();
        if identity[1] == "nul-suffix" {
            outside_ascii_scope.push(row[0].to_string());
            continue;
        }
        if active_identity != id[0] {
            let mut actual = Interp::with_native_core(
                std::rc::Rc::new(tcl_host_native::NativeHost::new()),
                crate::environment::profile_for_dialect(environment),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let target = actual.ensure_namespace(identity[0].as_bytes());
            push_namespace(&mut actual, target);
            interpreter = Some(actual);
            active_identity = id[0].to_string();
        }
        let interp = interpreter.as_mut().unwrap();
        let name: &[u8] = match identity[1].as_ref() {
            "empty" => b"",
            "absolute-root" => b"::",
            other => panic!("unrecognised oracle variant {other}"),
        };
        let member = if id[1] == "eval" {
            b"eval".as_slice()
        } else {
            assert!(matches!(id[1].as_ref(), "exists-before" | "exists-after"));
            b"exists".as_slice()
        };
        let mut owners = vec![
            obj::Owned::fresh(obj::new_string_bytes(b"namespace")),
            obj::Owned::fresh(obj::new_string_bytes(member)),
            obj::Owned::fresh(obj::new_string_bytes(name)),
        ];
        if member == b"eval" {
            owners.push(obj::Owned::fresh(obj::new_string_bytes(
                b"namespace current",
            )));
        }
        let arguments: Vec<_> = owners.iter().map(obj::Owned::as_ptr).collect();
        let code = interp.dispatch(&arguments);
        let completion = crate::completion::capture_bytes(interp, code).unwrap();
        assert_eq!(
            completion.code.as_int().to_string(),
            row[1],
            "{environment}: {}",
            row[0]
        );
        assert_eq!(
            completion.result,
            row[2].as_bytes(),
            "{environment}: {}",
            row[0]
        );
        if code != Code::Ok {
            let options =
                tcl_syntax::list::split_list(std::str::from_utf8(&completion.options).unwrap())
                    .unwrap();
            let error_code = options
                .chunks_exact(2)
                .find(|pair| pair[0] == "-errorcode")
                .unwrap();
            assert_eq!(error_code[1], row[3], "{environment}: {}", row[0]);
        }
        assert!(interp.native_access_refusal().is_none());
        compared += 1;
    }
    assert_eq!(compared, 12);
    assert_eq!(outside_ascii_scope.len(), 6);
    eprintln!(
        "{environment}: 12 original ASCII namespace getter/creation rows; six Unicode-NUL STRING rows outside this direct ASCII receiver scope: {outside_ascii_scope:?}; errorInfo formatting untested"
    );
}

#[test]
fn original_empty_namespace_address_controls_match_five_c_oracles() {
    for (environment, observation) in [
        (
            "tcl8.4",
            include_str!(
                "../../../../../rust/tcl-registry/tests/data/source_contract_vectors/namespace-address-purpose/8.4.20/stdout.bin"
            ),
        ),
        (
            "tcl8.5",
            include_str!(
                "../../../../../rust/tcl-registry/tests/data/source_contract_vectors/namespace-address-purpose/8.5.19/stdout.bin"
            ),
        ),
        (
            "tcl8.6",
            include_str!(
                "../../../../../rust/tcl-registry/tests/data/source_contract_vectors/namespace-address-purpose/8.6.18/stdout.bin"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../../../rust/tcl-registry/tests/data/source_contract_vectors/namespace-address-purpose/9.0.4/stdout.bin"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../../../rust/tcl-registry/tests/data/source_contract_vectors/namespace-address-purpose/9.1.0/stdout.bin"
            ),
        ),
    ] {
        compare_empty_namespace_address_rows(environment, observation);
    }
}

#[test]
fn logical_namespace_creation_keeps_its_explicit_simulation_and_internal_owner() {
    use tcl_syntax::naming::{NamePolicyAuthority, NamePolicyProtocol};
    let mut interp = Interp::new();
    interp.set_dialect_profile(crate::environment::profile_for_dialect("irules"));
    assert!(interp.name_policy_protocol().is_none());
    let internal = interp.ensure_namespace(b"::logical_internal");
    assert_eq!(
        interp
            .namespaces()
            .find_namespace(GLOBAL, b"::logical_internal"),
        Some(internal)
    );
    assert!(interp.name_policy_protocol().is_none());
    assert!(
        interp.set_logical_name_provider(NamePolicyProtocol::authored_tcl(
            tcl_dialect::TclVersion::V8_4
        ))
    );
    let policy = interp.name_policy_protocol().unwrap();
    assert_eq!(policy.authority(), NamePolicyAuthority::AuthoredSimulation);
    assert!(interp
        .native_invocation_dialect()
        .native_name_protocol()
        .is_none());
    assert!(interp.set_logical_source_word_provider(
        tcl_registry::invocation_words::LogicalSourceWordProvider::Tcl84CoreSimulation,
        crate::environment::profile_for_dialect("tcl8.6"),
    ));
    let created = interp.ns_eval_framed(b"::logical_created", b"", None, false, None, None);
    assert_eq!(created, Code::Ok);
    assert_eq!(interp.name_policy_protocol(), Some(policy));
    assert!(interp
        .namespaces()
        .find_namespace(GLOBAL, b"::logical_created")
        .is_some());
    assert!(interp
        .native_invocation_dialect()
        .native_name_protocol()
        .is_none());
}

mod selected_report_parts_tests {
    use super::*;
    use crate::obj::Owned;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    use tcl_core_types::{ByteNamespacePath, NameBytes};
    use tcl_runtime_api::NativeNamespaceReportParts;

    fn native(profile: &str) -> Interp {
        Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect(profile),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap()
    }
    fn report(
        actual: &mut Interp,
        namespace: NsId,
    ) -> Result<NativeNamespaceReportParts, tcl_cmd_core::CmdError> {
        tcl_runtime_api::Namespaces::namespace_report_parts_checked(
            actual,
            tcl_runtime_api::NsId(u32::try_from(namespace).unwrap()),
        )
    }

    #[test]
    fn selected_c_namespace_reports_preserve_actual_components_across_display_collisions_and_retirement(
    ) {
        // Software storage geometry: constructed report data supplies no native input or identity proof.
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut actual = native(profile);
            let first = [
                NameBytes::from(b"a::b".as_slice()),
                NameBytes::from(b"leaf\xff".as_slice()),
            ];
            let second = [
                NameBytes::from(b"a".as_slice()),
                NameBytes::from(b"b".as_slice()),
                NameBytes::from(b"leaf\xff".as_slice()),
            ];
            let left = actual
                .namespaces_mut()
                .ensure_namespace_at_components(GLOBAL, &first);
            let right = actual
                .namespaces_mut()
                .ensure_namespace_at_components(GLOBAL, &second);
            assert_ne!(left, right);
            assert_eq!(
                actual.ns_qualified_name(left),
                actual.ns_qualified_name(right)
            );
            let left_data = report(&mut actual, left).unwrap();
            let right_data = report(&mut actual, right).unwrap();
            assert_eq!(
                left_data,
                NativeNamespaceReportParts::CConstructed(ByteNamespacePath::from_segments(
                    first.clone()
                ))
            );
            assert_eq!(
                right_data,
                NativeNamespaceReportParts::CConstructed(ByteNamespacePath::from_segments(second))
            );
            assert_ne!(left_data, right_data);
            actual.delete_namespace_by_id(left);
            assert!(report(&mut actual, left).is_err());
            assert_eq!(
                left_data,
                NativeNamespaceReportParts::CConstructed(ByteNamespacePath::from_segments(first))
            );
        }
    }

    #[test]
    fn selected_c_namespace_report_retains_constructed_path_of_actual_dying_activation() {
        let mut actual = native("tcl8.6");
        let target = actual.ensure_global_namespace(b"::held::child");
        let before = report(&mut actual, target).unwrap();
        let saved = actual.current_ns();
        actual.enter_namespace_activation(target);
        actual.frames.borrow_mut().push_namespace(target);
        actual.set_current_ns(target);
        actual.delete_namespace_by_id(target);
        assert!(!actual.namespaces().namespace_is_live(target));
        assert_eq!(report(&mut actual, target).unwrap(), before);
        let popped = actual.pop_native_call_frame();
        actual.set_current_ns(saved);
        actual.leave_namespace_activation(popped);
        assert!(report(&mut actual, target).is_err());
    }

    thread_local! {
        static ACTUAL: RefCell<Option<crate::interp::WeakInterp>> = const { RefCell::new(None) };
        static ROLE: RefCell<Option<Rc<Owned>>> = const { RefCell::new(None) };
        static READS: Cell<usize> = const { Cell::new(0) };
        static MODE: Cell<u8> = const { Cell::new(0) };
        static TARGET: Cell<NsId> = const { Cell::new(GLOBAL) };
        static FIRST: RefCell<Option<tcl_runtime_api::NativeExecutionError>> = const { RefCell::new(None) };
    }
    struct Cleanup;
    impl Drop for Cleanup {
        fn drop(&mut self) {
            ACTUAL.with(|actual| actual.borrow_mut().take());
            ROLE.with(|role| role.borrow_mut().take());
        }
    }
    extern "C" fn update(original: *mut TclObj) {
        READS.with(|reads| reads.set(reads.get() + 1));
        unsafe { obj::set_native_updater_string_rep(original, b"branch::leaf\0\xff", false) };
        let mode = MODE.with(Cell::get);
        if mode == 0 {
            return;
        }
        let mut actual = ACTUAL
            .with(|owner| {
                owner
                    .borrow()
                    .as_ref()
                    .and_then(crate::interp::WeakInterp::upgrade)
            })
            .unwrap();
        actual.set_result_bytes(b"REACHED REPORT CALLBACK");
        match mode {
            1 => {
                actual.refuse_host_command("original namespace report first cause");
                FIRST.with(|first| *first.borrow_mut() = actual.native_execution_refusal());
            }
            2 => {
                let original = actual.runtime_context();
                let mut changed = original.clone();
                changed
                    .packages
                    .push(("report-parts-control".into(), "1.0".into()));
                actual.pin_context(&changed).unwrap();
                actual.pin_context(&original).unwrap();
            }
            3 => actual.set_current_ns(TARGET.with(Cell::get)),
            4 => {
                let current = actual.current_ns();
                actual.frames.borrow_mut().push(current);
            }
            5 => {
                let retired = ROLE.with(|role| role.borrow_mut().take());
                drop(retired);
            }
            _ => unreachable!(),
        }
    }
    static ORIGINAL: obj::TclObjType = obj::TclObjType {
        name: c"originalNamespaceReportParts".as_ptr(),
        free_int_rep_proc: None,
        dup_int_rep_proc: None,
        update_string_proc: Some(update),
        set_from_any_proc: None,
    };
    fn jim(mode: u8) -> (Interp, Cleanup, NsId, Owned) {
        ACTUAL.with(|actual| actual.borrow_mut().take());
        ROLE.with(|role| role.borrow_mut().take());
        READS.with(|reads| reads.set(0));
        MODE.with(|selected| selected.set(mode));
        FIRST.with(|first| first.borrow_mut().take());
        let actual = native("jimtcl");
        let holder = Owned::fresh(obj::alloc_typed(&ORIGINAL, 0));
        crate::native_source::bind_context(
            holder.as_ptr(),
            &actual.native_jim_object_context().unwrap(),
        )
        .unwrap();
        let target = actual
            .namespaces_mut()
            .retain_unmaterialized_jim_namespace(Owned::retain(holder.as_ptr()));
        let role = actual
            .namespaces_mut()
            .take_jim_namespace_owner(target)
            .unwrap();
        ROLE.with(|owner| *owner.borrow_mut() = Some(role));
        TARGET.with(|selected| selected.set(target));
        ACTUAL.with(|owner| *owner.borrow_mut() = Some(actual.downgrade()));
        (actual, Cleanup, target, holder)
    }

    #[test]
    fn selected_jim_report_reaches_original_holder_without_switching_caller_or_retaining_native_role(
    ) {
        let (mut actual, _cleanup, target, holder) = jim(0);
        let caller = actual.current_ns();
        let count = unsafe { (*holder.as_ptr()).ref_count };
        actual.set_result_bytes(b"PRIOR");
        let prior = actual.result_obj();
        assert_ne!(caller, target);
        assert!(actual.namespaces().native_context_path(target).is_none());
        let snapshot = report(&mut actual, target).unwrap();
        assert_eq!(
            snapshot,
            NativeNamespaceReportParts::JimFlat(Rc::from(b"branch::leaf\0\xff".as_slice()))
        );
        assert_eq!(READS.with(Cell::get), 1);
        assert_eq!(actual.current_ns(), caller);
        assert_eq!(actual.result_obj(), prior);
        assert_eq!(unsafe { (*holder.as_ptr()).ref_count }, count);
        assert_eq!(report(&mut actual, target).unwrap(), snapshot);
        assert_eq!(READS.with(Cell::get), 1);
        ROLE.with(|role| role.borrow_mut().take());
        assert!(report(&mut actual, target).is_err());
        assert_eq!(
            snapshot,
            NativeNamespaceReportParts::JimFlat(Rc::from(b"branch::leaf\0\xff".as_slice()))
        );
    }

    #[test]
    fn selected_jim_report_checks_first_cause_original_caller_and_target_after_reached_getter() {
        for mode in [1, 2, 3, 4, 5] {
            let (mut actual, _cleanup, target, holder) = jim(mode);
            let error = report(&mut actual, target).unwrap_err();
            assert_eq!(READS.with(Cell::get), 1, "{mode}");
            assert_eq!(obj::bytes_of(holder.as_ptr()), b"branch::leaf\0\xff");
            assert_eq!(
                error.native_execution_refusal(),
                actual.native_execution_refusal().as_ref()
            );
            assert!(actual.native_execution_refusal().is_some());
            assert_eq!(actual.result_bytes(), b"REACHED REPORT CALLBACK");
            if mode == 1 {
                assert_eq!(
                    FIRST.with(|first| first.borrow().clone()),
                    actual.native_execution_refusal()
                );
            }
            if mode == 4 {
                actual.frames.borrow_mut().pop();
            }
            assert_eq!(actual.entered_native_operation_count(), 0);
        }
    }

    #[test]
    fn selected_report_parts_refuse_missing_issuer_and_absent_target_owner() {
        let mut logical = Interp::new();
        logical.set_dialect_profile(crate::environment::profile_for_dialect("irules"));
        assert!(logical.name_policy_protocol().is_none());
        assert!(report(&mut logical, GLOBAL).is_err());
        let mut actual = native("jimtcl");
        assert!(report(&mut actual, u32::MAX as usize).is_err());
        let mut actual = native("jimtcl");
        let foreign = native("jimtcl");
        let holder = Owned::fresh(obj::new_string_bytes(b"foreign::scope"));
        crate::native_source::bind_context(
            holder.as_ptr(),
            &foreign.native_jim_object_context().unwrap(),
        )
        .unwrap();
        let target = actual
            .namespaces_mut()
            .retain_unmaterialized_jim_namespace(Owned::retain(holder.as_ptr()));
        let _owner = actual
            .namespaces_mut()
            .take_jim_namespace_owner(target)
            .unwrap();
        assert!(report(&mut actual, target).is_err());
    }
}
