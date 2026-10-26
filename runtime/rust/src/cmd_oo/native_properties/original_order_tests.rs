// SPDX-License-Identifier: AGPL-3.0-or-later
//! Selected property-list references and temporary object-hash key stages.
use super::*;
use std::cell::Cell;

const ORIGINAL: &str = include_str!(
    "../../../tests/data/native_property_slot_reference_order_original/509b/providers/9.1.0/execute.stdout"
);

#[derive(Clone, Debug, PartialEq, Eq)]
struct Observation {
    role: usize,
    free: bool,
    references: [obj::TclSize; 4],
    strings: [bool; 4],
    updates: [usize; 4],
}

thread_local! {
    static OWNER: RefCell<Option<crate::interp::WeakInterp>> = const { RefCell::new(None) };
    static MEMBERS: Cell<[*mut TclObj; 4]> = const { Cell::new([core::ptr::null_mut(); 4]) };
    static ACTIVE: Cell<bool> = const { Cell::new(false) };
    static MODE: Cell<u8> = const { Cell::new(0) };
    static UPDATES: Cell<[usize; 4]> = const { Cell::new([0; 4]) };
    static OBSERVATIONS: RefCell<Vec<Observation>> = const { RefCell::new(Vec::new()) };
    static OUTSIDE_BORROW: Cell<bool> = const { Cell::new(false) };
}

fn observe(role: usize, free: bool) {
    let members = MEMBERS.with(Cell::get);
    let references = members.map(|member| {
        if member.is_null() || !obj::allocation_is_live(member) {
            -1
        } else {
            unsafe { (*member).ref_count }
        }
    });
    let strings = members.map(|member| {
        !member.is_null() && obj::allocation_is_live(member) && obj::has_string_rep(member)
    });
    OBSERVATIONS.with(|observations| {
        observations.borrow_mut().push(Observation {
            role,
            free,
            references,
            strings,
            updates: UPDATES.with(Cell::get),
        });
    });
}

extern "C" fn update(member: *mut TclObj) {
    let role = MEMBERS.with(|members| {
        members
            .get()
            .iter()
            .position(|original| *original == member)
            .unwrap()
    });
    UPDATES.with(|updates| {
        let mut counts = updates.get();
        counts[role] += 1;
        updates.set(counts);
    });
    if ACTIVE.with(Cell::get) {
        observe(role, false);
        if role == 1 && matches!(MODE.with(Cell::get), 3 | 4) {
            let mut interpreter =
                OWNER.with(|owner| owner.borrow().as_ref().unwrap().upgrade().unwrap());
            if MODE.with(Cell::get) == 3 {
                interpreter.refuse_host_command("first selected property hash getter");
            } else {
                let original = interpreter.runtime_context();
                let mut changed = original.clone();
                changed
                    .packages
                    .push(("property-hash-callback".to_owned(), "1.0".to_owned()));
                interpreter.pin_context(&changed).unwrap();
                interpreter.pin_context(&original).unwrap();
            }
        }
    }
    let bytes: &[u8] = match role {
        0 => b"old",
        1 | 2 => b"p",
        _ => b"q",
    };
    unsafe { obj::set_native_updater_string_rep(member, bytes, false) };
}

extern "C" fn free(member: *mut TclObj) {
    if !ACTIVE.with(Cell::get) {
        return;
    }
    let role = MEMBERS.with(|members| {
        members
            .get()
            .iter()
            .position(|original| *original == member)
            .unwrap()
    });
    observe(role, true);
    MEMBERS.with(|members| {
        let mut originals = members.get();
        originals[role] = core::ptr::null_mut();
        members.set(originals);
    });
    if role == 0 {
        let mut interpreter =
            OWNER.with(|owner| owner.borrow().as_ref().unwrap().upgrade().unwrap());
        OUTSIDE_BORROW.with(|outside| outside.set(interpreter.oo.try_borrow_mut().is_ok()));
        match MODE.with(Cell::get) {
            1 => {
                interpreter.refuse_host_command("first selected property retirement");
            }
            2 => {
                let original = interpreter.runtime_context();
                let mut changed = original.clone();
                changed
                    .packages
                    .push(("property-order-callback".to_owned(), "1.0".to_owned()));
                interpreter.pin_context(&changed).unwrap();
                interpreter.pin_context(&original).unwrap();
            }
            _ => {}
        }
    }
}

static MEMBER: obj::TclObjType = obj::TclObjType {
    name: c"originalPropertyOrderMember".as_ptr(),
    free_int_rep_proc: Some(free),
    dup_int_rep_proc: None,
    update_string_proc: Some(update),
    set_from_any_proc: None,
};

fn native(class: bool) -> (Interp, OoId) {
    ACTIVE.with(|active| active.set(false));
    MODE.with(|mode| mode.set(0));
    UPDATES.with(|updates| updates.set([0; 4]));
    OBSERVATIONS.with(|observations| observations.borrow_mut().clear());
    OUTSIDE_BORROW.with(|outside| outside.set(false));
    let mut interpreter = Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("tcl9.1"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    assert_eq!(
        interpreter.eval_str(
            b"oo::class create PropertyOrderClass; PropertyOrderClass create PropertyOrderObject"
        ),
        Code::Ok
    );
    let target = interpreter.oo_resolve_object(if class {
        b"PropertyOrderClass"
    } else {
        b"PropertyOrderObject"
    });
    OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
    (interpreter, target)
}

fn strings(interpreter: &Interp) -> tcl_syntax::native_string::NativeStringProtocol {
    interpreter
        .native_invocation_dialect()
        .native_property_lookup_protocol()
        .unwrap()
        .strings()
}

fn info_script(class: bool, writable: bool) -> &'static [u8] {
    match (class, writable) {
        (true, false) => b"info class properties PropertyOrderClass -all -readable",
        (true, true) => b"info class properties PropertyOrderClass -all -writable",
        (false, false) => b"info object properties PropertyOrderObject -all -readable",
        (false, true) => b"info object properties PropertyOrderObject -all -writable",
    }
}

fn members() -> [obj::Owned; 4] {
    let members = std::array::from_fn(|_| obj::Owned::fresh(obj::alloc_typed(&MEMBER, 0)));
    MEMBERS.with(|originals| originals.set(members.each_ref().map(obj::Owned::as_ptr)));
    members
}

fn original_fields(line: &str) -> BTreeMap<&str, &str> {
    line.split('\t')
        .skip(1)
        .filter_map(|field| field.split_once('='))
        .collect()
}

fn original_row(
    case: usize,
    phase: &str,
    stage: &str,
    role: Option<usize>,
) -> BTreeMap<&'static str, &'static str> {
    ORIGINAL
        .lines()
        .filter(|line| line.starts_with("ROW\t"))
        .find_map(|line| {
            let fields = original_fields(line);
            if fields["case"].parse::<usize>().unwrap() != case
                || fields["phase"] != phase
                || fields["stage"] != stage
            {
                return None;
            }
            if let Some(role) = role {
                let field = format!("m{role}_update");
                if fields[field.as_str()] != "1" {
                    return None;
                }
                // Later rows also contain the earlier updater count.
                if role < 3 && fields[format!("m{}_update", role + 1).as_str()] != "0" {
                    return None;
                }
            }
            Some(fields)
        })
        .expect("exact original case and reached stage")
}

fn assert_original(observation: &Observation, fields: &BTreeMap<&str, &str>) {
    for role in 1..4 {
        assert_eq!(
            observation.references[role],
            fields[format!("m{role}_refs").as_str()]
                .parse::<obj::TclSize>()
                .unwrap()
        );
        assert_eq!(
            observation.strings[role],
            fields[format!("m{role}_string").as_str()] == "1"
        );
        assert_eq!(
            observation.updates[role],
            fields[format!("m{role}_update").as_str()]
                .parse::<usize>()
                .unwrap()
        );
    }
}

#[test]
fn original_property_replacement_matches_incoming_hash_and_cache_reference_stages() {
    // naming.property.original-slot-storage-reference-order
    // docs/design/analysis/name-resolution-proofs/property-original-slot-storage-reference-order.md
    // Only matched C9.1 worker, held-cache and raw List stages; presentation is separate.
    for (case, class, writable, held) in [
        (0, true, false, false),
        (1, true, false, true),
        (2, false, false, false),
        (3, false, false, true),
        (4, true, true, false),
        (5, true, true, true),
    ] {
        let (mut interpreter, target) = native(class);
        let [old, first, duplicate, last] = members();
        let old_pointer = old.as_ptr();
        let old_lifetime = obj::NativeObjectLifetime::retain(old_pointer);
        let initial = obj::Owned::fresh(crate::list::new_list_obj_native(
            &[old_pointer],
            strings(&interpreter),
        ));
        set_property_slot(&mut interpreter, target, class, writable, initial.as_ptr()).unwrap();
        drop(initial);
        assert_eq!(interpreter.eval_str(info_script(class, writable)), Code::Ok);
        let cache = interpreter
            .native_all_property_header(target, class, writable)
            .unwrap();
        assert_eq!(cache.header, interpreter.get_obj_result());
        // Actual info issued the result role; the explicit caller hold is independent.
        let held_cache = held.then(|| obj::Owned::retain(cache.header));
        interpreter.reset_result();
        drop(old);
        let input = obj::Owned::fresh(crate::list::new_list_obj_native(
            &[first.as_ptr(), duplicate.as_ptr(), last.as_ptr()],
            strings(&interpreter),
        ));
        ACTIVE.with(|active| active.set(true));
        set_property_slot(&mut interpreter, target, class, writable, input.as_ptr()).unwrap();
        drop(input);
        let observations = OBSERVATIONS.with(|observations| observations.borrow().clone());
        let update_roles: Vec<_> = observations
            .iter()
            .filter(|stage| !stage.free)
            .map(|stage| stage.role)
            .collect();
        assert_eq!(update_roles, [1, 2, 3]);
        for observation in observations.iter().filter(|stage| !stage.free) {
            assert_original(
                observation,
                &original_row(
                    case,
                    "worker",
                    "inside-member-update-before-string",
                    Some(observation.role),
                ),
            );
        }
        if held {
            assert!(obj::allocation_is_live(old_pointer));
            assert!(!observations.iter().any(|stage| stage.free));
        } else {
            let retirement = observations.first().unwrap();
            assert!(retirement.free);
            assert_eq!(retirement.role, 0);
            assert_original(
                retirement,
                &original_row(case, "worker", "inside-member-free", None),
            );
            assert!(OUTSIDE_BORROW.with(Cell::get));
            assert!(!obj::allocation_is_live(old_pointer));
        }
        drop(held_cache);
        if held {
            let observations = OBSERVATIONS.with(|observations| observations.borrow().clone());
            let retirement = observations.last().unwrap();
            assert_eq!((retirement.role, retirement.free), (0, true));
            assert_original(
                retirement,
                &original_row(case, "cache-release", "inside-member-free", None),
            );
            assert!(OUTSIDE_BORROW.with(Cell::get));
        }
        assert!(!obj::allocation_is_live(old_pointer));
        let after = original_row(case, "worker", "after-public-slot-set", None);
        for (role, member) in [(1, &first), (2, &duplicate), (3, &last)] {
            assert_eq!(
                unsafe { (*member.as_ptr()).ref_count },
                after[format!("m{role}_refs").as_str()]
                    .parse::<obj::TclSize>()
                    .unwrap()
            );
        }
        assert_eq!(interpreter.eval_str(info_script(class, writable)), Code::Ok);
        let new_cache = interpreter
            .native_all_property_header(target, class, writable)
            .unwrap();
        assert_eq!(new_cache.header, interpreter.get_obj_result());
        let raw =
            crate::list::list_elements_native_checked(new_cache.header, strings(&interpreter))
                .unwrap();
        assert_eq!(raw, [first.as_ptr(), last.as_ptr()]);
        assert!(!obj::has_string_rep(new_cache.header));
        let original_list = ORIGINAL
            .lines()
            .filter(|line| line.starts_with("LIST\t"))
            .find_map(|line| {
                let fields = original_fields(line);
                (fields["case"].parse::<usize>().unwrap() == case && fields["phase"] == "post-info")
                    .then_some(fields)
            })
            .unwrap();
        assert_eq!(raw.len(), original_list["count"].parse::<usize>().unwrap());
        assert_eq!(
            unsafe { (*new_cache.header).ref_count },
            original_list["header_refs"]
                .parse::<obj::TclSize>()
                .unwrap()
        );
        assert_eq!(original_list["header_string"], "0");
        assert_eq!(original_list["member0_role"], "1");
        assert_eq!(original_list["member1_role"], "3");
        for (index, member) in raw.iter().enumerate() {
            assert_eq!(
                unsafe { (**member).ref_count },
                original_list[format!("member{index}_refs").as_str()]
                    .parse::<obj::TclSize>()
                    .unwrap()
            );
        }
        ACTIVE.with(|active| active.set(false));
        interpreter.reset_result();
        drop(new_cache);
        drop(cache);
        drop(old_lifetime);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}

#[test]
fn original_property_old_retirement_refuses_before_any_hash_getter_or_adoption() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Genuine software Host/ABA callbacks; original509b contains no such callback mode.
    for mode in [1, 2] {
        let (mut interpreter, target) = native(true);
        let [old, first, duplicate, last] = members();
        let pointer = old.as_ptr();
        let lifetime = obj::NativeObjectLifetime::retain(pointer);
        let initial = obj::Owned::fresh(crate::list::new_list_obj_native(
            &[pointer],
            strings(&interpreter),
        ));
        set_property_slot(&mut interpreter, target, true, false, initial.as_ptr()).unwrap();
        drop(initial);
        drop(old);
        let input = obj::Owned::fresh(crate::list::new_list_obj_native(
            &[first.as_ptr(), duplicate.as_ptr(), last.as_ptr()],
            strings(&interpreter),
        ));
        interpreter.set_result_bytes(b"PRIOR");
        let prior = interpreter.get_obj_result();
        let epoch = interpreter.oo.borrow().property_foundation_epoch;
        MODE.with(|selected| selected.set(mode));
        ACTIVE.with(|active| active.set(true));
        let failure = set_property_slot(&mut interpreter, target, true, false, input.as_ptr())
            .err()
            .unwrap_or_else(|| panic!("reached old retirement refuses"));
        let first_cause = interpreter.native_execution_refusal().unwrap();
        assert_eq!(failure.native_execution_refusal(), Some(&first_cause));
        let observations = OBSERVATIONS.with(|observations| observations.borrow().clone());
        assert_eq!(observations.len(), 1);
        let retirement = &observations[0];
        assert_eq!((retirement.role, retirement.free), (0, true));
        assert_eq!(&retirement.references[1..], &[3, 3, 3]);
        assert_eq!(&retirement.strings[1..], &[false, false, false]);
        assert_eq!(&retirement.updates[1..], &[0, 0, 0]);
        assert!(OUTSIDE_BORROW.with(Cell::get));
        assert!(!obj::allocation_is_live(pointer));
        let state = interpreter.oo.borrow();
        assert_eq!(
            state.classes[&target].readable_properties,
            [b"old".to_vec()]
        );
        assert!(!state.property_members.contains_key(&(target, true, false)));
        assert_eq!(state.property_foundation_epoch, epoch);
        drop(state);
        for member in [&first, &duplicate, &last] {
            assert_eq!(unsafe { (*member.as_ptr()).ref_count }, 2);
            assert!(!obj::has_string_rep(member.as_ptr()));
        }
        assert_eq!(interpreter.get_obj_result(), prior);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
        ACTIVE.with(|active| active.set(false));
        drop(lifetime);
    }
}

#[test]
fn original_property_hash_refusal_preserves_reached_string_and_stops_later_keys() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Software-only refusal after the genuine first selected hash String getter.
    for mode in [3, 4] {
        let (mut interpreter, target) = native(true);
        let [old, first, duplicate, last] = members();
        let initial = obj::Owned::fresh(crate::list::new_list_obj_native(
            &[old.as_ptr()],
            strings(&interpreter),
        ));
        set_property_slot(&mut interpreter, target, true, false, initial.as_ptr()).unwrap();
        drop(initial);
        drop(old);
        let input = obj::Owned::fresh(crate::list::new_list_obj_native(
            &[first.as_ptr(), duplicate.as_ptr(), last.as_ptr()],
            strings(&interpreter),
        ));
        interpreter.set_result_bytes(b"PRIOR");
        let prior = interpreter.get_obj_result();
        let epoch = interpreter.oo.borrow().property_foundation_epoch;
        MODE.with(|selected| selected.set(mode));
        ACTIVE.with(|active| active.set(true));
        let failure = set_property_slot(&mut interpreter, target, true, false, input.as_ptr())
            .err()
            .unwrap_or_else(|| panic!("first hash getter refuses"));
        let first_cause = interpreter.native_execution_refusal().unwrap();
        assert_eq!(failure.native_execution_refusal(), Some(&first_cause));
        let observations = OBSERVATIONS.with(|observations| observations.borrow().clone());
        assert_eq!(observations.len(), 2);
        assert_eq!((observations[0].role, observations[0].free), (0, true));
        assert_eq!((observations[1].role, observations[1].free), (1, false));
        assert_eq!(&observations[1].references[1..], &[3, 3, 3]);
        assert_eq!(&observations[1].strings[1..], &[false, false, false]);
        assert_eq!(UPDATES.with(Cell::get), [1, 1, 0, 0]);
        assert_eq!(obj::bytes_of(first.as_ptr()), b"p");
        assert!(!obj::has_string_rep(duplicate.as_ptr()));
        assert!(!obj::has_string_rep(last.as_ptr()));
        for member in [&first, &duplicate, &last] {
            assert_eq!(unsafe { (*member.as_ptr()).ref_count }, 2);
        }
        let state = interpreter.oo.borrow();
        assert_eq!(
            state.classes[&target].readable_properties,
            [b"old".to_vec()]
        );
        assert!(!state.property_members.contains_key(&(target, true, false)));
        assert_eq!(state.property_foundation_epoch, epoch);
        drop(state);
        assert_eq!(interpreter.get_obj_result(), prior);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
        ACTIVE.with(|active| active.set(false));
    }
}
