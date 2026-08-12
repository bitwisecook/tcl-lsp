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

//! Crash containment, quarantine, and per-pack isolation.
//!
//! "A spec must never be able to take the LSP down" is a guarantee about the
//! *host*, not about any engine, so these tests drive the host with a
//! deliberately hostile engine: one that panics, errors, or blows its budget
//! on command. That is the only way to assert the policy rather than the
//! absence of a VM bug — and it exercises the same code path a real panic
//! would.

use std::cell::Cell;
use std::rc::Rc;

use tcl_engine_api::{Budget, CompileUnit, Engine, EngineError, HostCommand, Value};
use tcl_registry::invocation_words::InvocationWordKind;
use tcl_registry::pack_hooks::{
    self, HookAnswer, HookCall, HookFamily, HookWord, PackHookHost,
};
use tcl_spec_hooks::{CrashKind, HookHost, HookProgram, PackPrograms};

/// What the hostile engine does when a body is invoked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Behaviour {
    Fold,
    Panic,
    Error,
    BurnBudget,
}

fn behaviour_of(body: &str) -> Behaviour {
    match body {
        "panic" => Behaviour::Panic,
        "error" => Behaviour::Error,
        "burn" => Behaviour::BurnBudget,
        _ => Behaviour::Fold,
    }
}

/// An engine whose only job is to misbehave on demand.
struct HostileEngine {
    folder: Option<Rc<dyn HostCommand>>,
    invocations: Rc<Cell<u32>>,
}

impl Engine for HostileEngine {
    type Handle = Behaviour;

    fn name(&self) -> &'static str {
        "hostile"
    }

    fn define_command(
        &mut self,
        name: &str,
        command: Rc<dyn HostCommand>,
    ) -> Result<(), EngineError> {
        if name == "fold" {
            self.folder = Some(command);
        }
        Ok(())
    }

    fn restrict_commands(&mut self, _allowed: &[&str]) -> Result<(), EngineError> {
        Ok(())
    }

    fn compile(&mut self, unit: CompileUnit<'_>) -> Result<Self::Handle, EngineError> {
        Ok(behaviour_of(unit.body))
    }

    fn invoke(
        &mut self,
        handle: &Self::Handle,
        _arguments: &[Value],
    ) -> Result<Value, EngineError> {
        self.invocations.set(self.invocations.get() + 1);
        match handle {
            Behaviour::Fold => {
                if let Some(folder) = &self.folder {
                    folder.invoke(&[Value::string("folded")])?;
                }
                Ok(Value::Empty)
            }
            Behaviour::Panic => panic!("the hook body panicked"),
            Behaviour::Error => Err(EngineError::Script {
                message: "bad index".to_owned(),
                code: None,
            }),
            Behaviour::BurnBudget => Err(EngineError::BudgetExceeded(
                tcl_engine_api::BudgetKind::Commands,
            )),
        }
    }

    fn set_budget(&mut self, _budget: Budget) -> Result<(), EngineError> {
        Ok(())
    }

    fn commands_spent(&self) -> Option<u64> {
        None
    }
}

fn host(invocations: &Rc<Cell<u32>>) -> HookHost<HostileEngine> {
    let invocations = Rc::clone(invocations);
    HookHost::new(move || HostileEngine {
        folder: None,
        invocations: Rc::clone(&invocations),
    })
}

fn call<'w>(words: &'w [HookWord<'w>]) -> HookCall<'w> {
    HookCall {
        words,
        version: None,
        in_event_body: false,
        option: None,
    }
}

fn literal<'w>(value: &'w str) -> HookWord<'w> {
    HookWord {
        value,
        kind: InvocationWordKind::Literal,
    }
}

#[test]
fn a_panicking_hook_abstains_and_is_quarantined_with_a_record() {
    let invocations = Rc::new(Cell::new(0));
    let host = host(&invocations);
    let installed = host.load_pack(
        PackPrograms {
            pack: "badlib".to_owned(),
            content_hash: "deadbeef".to_owned(),
            dsl_version: "1".to_owned(),
            programs: vec![HookProgram::new(
                "badlib::boom",
                HookFamily::ConstFold,
                "panic",
            )],
        },
    );
    let slot = installed[0].slot.expect("installed");

    let words = [literal("x")];
    // The panic is caught and converted to the family's silence.
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let answer = host.invoke(slot, &call(&words));
    std::panic::set_hook(previous);
    assert_eq!(answer, HookAnswer::Abstain);
    assert_eq!(invocations.get(), 1);

    // Quarantined: a second call never reaches the engine again, so one bad
    // hook cannot re-crash on every keystroke.
    assert!(host.is_quarantined(slot));
    assert_eq!(host.invoke(slot, &call(&words)), HookAnswer::Abstain);
    assert_eq!(invocations.get(), 1);

    let [record] = &host.crash_records()[..] else {
        panic!("exactly one crash record");
    };
    assert_eq!(record.kind, CrashKind::Panic);
    assert_eq!(record.pack, "badlib");
    assert_eq!(record.content_hash, "deadbeef");
    assert_eq!(record.family, HookFamily::ConstFold);
    assert!(record.hook.contains("badlib::boom"));
    assert_eq!(record.detail, "the hook body panicked");
    // Shapes, never words: the record must not carry the user's source.
    assert_eq!(record.word_shapes, vec![InvocationWordKind::Literal]);
    assert!(!record.headline().contains('x'));
}

#[test]
fn a_budget_blowout_is_recorded_and_quarantined_but_named_differently() {
    let invocations = Rc::new(Cell::new(0));
    let host = host(&invocations);
    let installed = host.load_pack(
        PackPrograms::new("slowlib").with(HookProgram::new(
            "slowlib::spin",
            HookFamily::ConstFold,
            "burn",
        )),
    );
    let slot = installed[0].slot.expect("installed");
    let words = [literal("x")];
    assert_eq!(host.invoke(slot, &call(&words)), HookAnswer::Abstain);
    assert!(host.is_quarantined(slot));
    let [record] = &host.crash_records()[..] else {
        panic!("exactly one crash record");
    };
    assert_eq!(record.kind, CrashKind::CommandBudget);
}

#[test]
fn an_erroring_hook_abstains_forever_but_is_logged_once() {
    let invocations = Rc::new(Cell::new(0));
    let host = host(&invocations);
    let installed = host.load_pack(
        PackPrograms::new("mylib").with(HookProgram::new(
            "mylib::wrong",
            HookFamily::ConstFold,
            "error",
        )),
    );
    let slot = installed[0].slot.expect("installed");
    let words = [literal("x")];
    for _ in 0..5 {
        assert_eq!(host.invoke(slot, &call(&words)), HookAnswer::Abstain);
    }
    // An error is not a crash: the hook keeps running (it may succeed on
    // other inputs), and the log records it once.
    assert!(!host.is_quarantined(slot));
    assert_eq!(invocations.get(), 5);
    assert!(host.crash_records().is_empty());
    let log = host.error_log();
    assert_eq!(log.len(), 1, "{log:?}");
    assert!(log[0].contains("bad index"), "{log:?}");
}

#[test]
fn a_crash_in_one_pack_leaves_another_pack_working() {
    let invocations = Rc::new(Cell::new(0));
    let host = host(&invocations);
    let bad = host.load_pack(
        PackPrograms::new("badlib").with(HookProgram::new(
            "badlib::boom",
            HookFamily::ConstFold,
            "panic",
        )),
    );
    let good = host.load_pack(
        PackPrograms::new("goodlib").with(HookProgram::new(
            "goodlib::ok",
            HookFamily::ConstFold,
            "fold",
        )),
    );
    let words = [literal("x")];
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let _ = host.invoke(bad[0].slot.expect("installed"), &call(&words));
    std::panic::set_hook(previous);

    let answer = host.invoke(good[0].slot.expect("installed"), &call(&words));
    assert_eq!(
        answer,
        HookAnswer::Fold("folded".to_owned()),
        "one library's bad hook must cost only that library its hooks"
    );
}

#[test]
fn a_panic_costs_the_pack_its_other_hooks_and_nothing_more() {
    let invocations = Rc::new(Cell::new(0));
    let host = host(&invocations);
    let installed = host.load_pack(
        PackPrograms::new("badlib")
            .with(HookProgram::new(
                "badlib::boom",
                HookFamily::ConstFold,
                "panic",
            ))
            .with(HookProgram::new(
                "badlib::fine",
                HookFamily::ConstFold,
                "fold",
            )),
    );
    let words = [literal("x")];
    let sibling = installed[1].slot.expect("installed");
    assert_eq!(
        host.invoke(sibling, &call(&words)),
        HookAnswer::Fold("folded".to_owned()),
        "the sibling works before the crash"
    );

    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let _ = host.invoke(installed[0].slot.expect("installed"), &call(&words));
    std::panic::set_hook(previous);

    // A caught panic leaves the engine's own state unproven, so the pack —
    // and only the pack — stops answering.
    assert_eq!(host.invoke(sibling, &call(&words)), HookAnswer::Abstain);
    assert!(host.is_quarantined(sibling));
}

#[test]
fn a_fold_never_runs_on_a_call_carrying_a_dynamic_word() {
    let invocations = Rc::new(Cell::new(0));
    let host = host(&invocations);
    let installed = host.load_pack(
        PackPrograms::new("mylib").with(HookProgram::new(
            "mylib::fold",
            HookFamily::ConstFold,
            "fold",
        )),
    );
    let slot = installed[0].slot.expect("installed");
    let words = [
        literal("a"),
        HookWord {
            value: "",
            kind: InvocationWordKind::Dynamic,
        },
    ];
    assert_eq!(host.invoke(slot, &call(&words)), HookAnswer::Abstain);
    assert_eq!(
        invocations.get(),
        0,
        "the precondition is enforced before the body runs"
    );
}

#[test]
fn an_unknown_slot_abstains() {
    let invocations = Rc::new(Cell::new(0));
    let host = host(&invocations);
    let orphan = pack_hooks::allocate(HookFamily::ConstFold, &pack_hooks::HookInputs::unrestricted())
        .expect("a slot");
    let words = [literal("x")];
    assert_eq!(host.invoke(orphan, &call(&words)), HookAnswer::Abstain);
}
