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
