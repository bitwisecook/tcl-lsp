// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual binding/declaration identity and active/future namespace relocation.

use super::*;

const OBSERVATIONS: &str =
    include_str!("../../tests/data/native_procedure_relocation/observations.tsv");
const SETUP: &[u8] = b"namespace eval A {proc p {move destination} {set before [namespace current]; if {$move} {rename ::A::p $destination; observe}; list $before [namespace current]}}; namespace eval B {}";
const DESTINATIONS: &[&[u8]] = &[b"::B::q", b"q", b"B::q", b"q", b"::q", b"B::q"];

struct Observer {
    generation: u64,
    namespace: NsId,
    jim_namespace: Option<obj::Owned>,
    frames: Vec<(bool, bool)>,
}
thread_local! {
    static OBSERVER: RefCell<Option<Observer>> = const { RefCell::new(None) };
}
struct ObserverScope;
impl Drop for ObserverScope {
    fn drop(&mut self) {
        OBSERVER.with(|observer| {
            observer.borrow_mut().take();
        });
    }
}
fn observe(interp: &mut Interp, _arguments: &[*mut TclObj]) -> Code {
    OBSERVER.with(|state| {
        let mut state = state.borrow_mut();
        let state = state.as_mut().expect("scoped relocation observer");
        let (_, Command::Proc(binding)) = interp
            .namespaces()
            .command_by_generation(state.generation)
            .expect("same live procedure node")
        else {
            panic!("actual procedure node")
        };
        let declaration = binding.declaration();
        let (frame_same, declaration_same) = if let Some(original) = &state.jim_namespace {
            let (_, _, frame_namespace) = interp
                .frames
                .borrow()
                .jim_activation_objects()
                .expect("actual Jim procedure frame");
            let location = declaration.location();
            (
                frame_namespace == original.as_ptr(),
                location.jim_namespace.as_ref().unwrap().as_ptr() == original.as_ptr(),
            )
        } else {
            (
                interp.current_ns.get() == state.namespace,
                declaration.namespace() == state.namespace,
            )
        };
        state.frames.push((frame_same, declaration_same));
    });
    Code::Ok
}
fn unhex(bytes: &str) -> Vec<u8> {
    assert!(bytes.len().is_multiple_of(2));
    bytes
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
fn procedure(interp: &Interp, name: &[u8]) -> (u64, NativeProcedureCommand) {
    let namespaces = interp.namespaces();
    let generation = namespaces.resolve_generation(GLOBAL, name).unwrap();
    let (_, Command::Proc(binding)) = namespaces.command_by_generation(generation).unwrap() else {
        panic!("original procedure binding")
    };
    (generation, binding)
}
#[test]
fn relocation_retains_native_binding_and_declaration_with_separate_active_namespace() {
    // Native proof: naming.procedure.original-relocation-namespace-and-identity
    // docs/design/analysis/name-resolution-proofs/procedure-original-relocation-namespace-and-identity.md
    let mut matched = 0;
    for (profile, native) in [
        ("tcl8.4", "8.4.20"),
        ("tcl8.5", "8.5.19"),
        ("tcl8.6", "8.6.18"),
        ("tcl9.0", "9.0.4"),
        ("tcl9.1", "9.1.0"),
        ("jim", "jim"),
    ] {
        for (case, destination) in DESTINATIONS.iter().enumerate() {
            let prefix = format!("{native}\t{case}\t");
            let fields: Vec<_> = OBSERVATIONS
                .lines()
                .find(|row| row.starts_with(&prefix))
                .unwrap()
                .split('\t')
                .collect();
            let mut interp = Interp::with_native_core(
                default_host(),
                crate::environment::profile_for_dialect(profile),
                tcl_registry::special_vars::NativeBootstrapInputs {
                    package_path: Vec::new(),
                    default_library: None,
                },
            )
            .unwrap();
            interp.register_builtin(b"observe", observe);
            assert_eq!(
                interp.eval_str(SETUP),
                Code::Ok,
                "{profile}: {:?}",
                interp.result_bytes()
            );
            let (generation, original) = procedure(&interp, b"::A::p");
            let declaration = original.declaration();
            let location = declaration.location();
            let old_jim_namespace = location
                .jim_namespace
                .as_ref()
                .map(|owner| obj::Owned::retain(owner.as_ptr()));
            OBSERVER.with(|observer| {
                *observer.borrow_mut() = Some(Observer {
                    generation,
                    namespace: location.namespace,
                    jim_namespace: old_jim_namespace,
                    frames: Vec::new(),
                })
            });
            let _scope = ObserverScope;
            let mut first = if case < 3 {
                b"::A::p 1 ".to_vec()
            } else {
                b"rename ::A::p ".to_vec()
            };
            first.extend_from_slice(destination);
            assert_eq!(
                interp.eval_str(&first).as_int(),
                fields[3].parse::<i64>().unwrap(),
                "{profile} case{case}"
            );
            assert_eq!(
                interp.result_bytes(),
                unhex(fields[4]),
                "{profile} case{case} active result"
            );
            let published = unhex(fields[2]);
            let (moved_generation, moved) = procedure(&interp, &published);
            assert_eq!(fields[8], "1", "native same binding receipt");
            assert_eq!(moved_generation, generation, "{profile} case{case} node");
            assert!(
                original.is_same_binding(&moved),
                "{profile} case{case} binding"
            );
            assert!(
                Rc::ptr_eq(&declaration, &moved.declaration()),
                "{profile} case{case} declaration"
            );
            OBSERVER.with(|observer| {
                let observer = observer.borrow();
                let observer = observer.as_ref().unwrap();
                let location = moved.declaration().location();
                let same_holder = observer
                    .jim_namespace
                    .as_ref()
                    .map_or(location.namespace == observer.namespace, |old| {
                        location.jim_namespace.as_ref().unwrap().as_ptr() == old.as_ptr()
                    });
                assert_eq!(
                    same_holder,
                    fields[7] == "1",
                    "{profile} case{case} namespace holder"
                );
                if case < 3 {
                    let start = if native == "jim" { 9 } else { 10 };
                    assert_eq!(
                        observer.frames.as_slice(),
                        &[(fields[start] == "1", fields[start + 1] == "1")],
                        "{profile} case{case} callback namespace"
                    );
                } else {
                    assert!(observer.frames.is_empty());
                }
            });
            let mut future = published;
            future.extend_from_slice(b" 0 unused");
            assert_eq!(
                interp.eval_str(&future).as_int(),
                fields[5].parse::<i64>().unwrap(),
                "{profile} case{case} future status"
            );
            assert_eq!(
                interp.result_bytes(),
                unhex(fields[6]),
                "{profile} case{case} future namespace"
            );
            matched += 1;
        }
    }
    assert_eq!(matched, 36);
}

#[test]
fn relocation_of_original_counted_names_matches_native_binding_identity() {
    // Native proof: naming.procedure.original-counted-relocation-identity
    // docs/design/analysis/name-resolution-proofs/procedure-original-counted-relocation-identity.md
    let original_names: &[&[u8]] = &[
        b"::A::p\xff",
        b"::A::p\0tail",
        b"p\xff",
        b"::A::p\xff\0tail",
    ];
    let destinations: &[&[u8]] = &[
        b"::B::q\xff",
        b"::B::q\0tail",
        b"::B::q\xff",
        b"::B::q\xff\0tail",
    ];
    let observations =
        include_str!("../../tests/data/native_procedure_relocation/raw-observations.tsv");
    let mut matched = 0;
    for (profile, native) in [
        ("tcl8.4", "8.4.20"),
        ("tcl8.5", "8.5.19"),
        ("tcl8.6", "8.6.18"),
        ("tcl9.0", "9.0.4"),
        ("tcl9.1", "9.1.0"),
        ("jim", "jim"),
    ] {
        for case in 0..original_names.len() {
            let prefix = format!("{native}\t{case}\t");
            let fields = observations
                .lines()
                .find(|line| line.starts_with(&prefix))
                .unwrap()
                .split('\t')
                .collect::<Vec<_>>();
            let mut interp = Interp::with_native_core(
                default_host(),
                crate::environment::profile_for_dialect(profile),
                tcl_registry::special_vars::NativeBootstrapInputs {
                    package_path: Vec::new(),
                    default_library: None,
                },
            )
            .unwrap();
            assert_eq!(
                interp.eval_str(b"namespace eval A {};namespace eval B {}"),
                Code::Ok
            );
            let declaration_argv = [
                b"proc".as_slice(),
                original_names[case],
                b"",
                b"namespace current",
            ]
            .map(|bytes| obj::Owned::fresh(obj::new_string_bytes(bytes)));
            assert_eq!(
                interp.dispatch_invoke(
                    &declaration_argv
                        .iter()
                        .map(obj::Owned::as_ptr)
                        .collect::<Vec<_>>()
                ),
                Code::Ok,
                "{profile} case{case} declaration"
            );
            let (Command::Proc(original), old_generation) = interp
                .resolve_original_command(declaration_argv[1].as_ptr())
                .unwrap()
                .unwrap()
            else {
                panic!("actual original procedure")
            };
            let declaration = original.declaration();
            let rename = obj::Owned::fresh(obj::new_string_bytes(b"rename"));
            let destination = obj::Owned::fresh(obj::new_string_bytes(destinations[case]));
            assert_eq!(
                interp
                    .dispatch_invoke(&[
                        rename.as_ptr(),
                        declaration_argv[1].as_ptr(),
                        destination.as_ptr()
                    ])
                    .as_int(),
                fields[2].parse::<i64>().unwrap(),
                "{profile} case{case} rename"
            );
            let (Command::Proc(moved), generation) = interp
                .resolve_original_command(destination.as_ptr())
                .unwrap()
                .unwrap()
            else {
                panic!("actual moved procedure")
            };
            assert_eq!(fields[3], "1");
            assert_eq!(
                generation, old_generation,
                "{profile} case{case} generation"
            );
            assert!(
                original.is_same_binding(&moved),
                "{profile} case{case} binding"
            );
            assert!(
                Rc::ptr_eq(&declaration, &moved.declaration()),
                "{profile} case{case} declaration"
            );
            assert_eq!(
                interp.dispatch_invoke(&[destination.as_ptr()]).as_int(),
                fields[4].parse::<i64>().unwrap(),
                "{profile} case{case} future status"
            );
            assert_eq!(
                interp.result_bytes(),
                unhex(fields[5]),
                "{profile} case{case} future namespace"
            );
            matched += 1;
        }
    }
    assert_eq!(matched, 24);
}
