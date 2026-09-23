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
use tcl_registry::pack_hooks::{self, HookAnswer, HookCall, HookFamily, HookWord, PackHookHost};
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

    fn confine_stores(&mut self) -> Result<(), EngineError> {
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
        constraints: None,
        dialect: None,
        targets: &[],
        budget: tcl_registry::value_transfer::ImplementationBudget::default(),
        depends: &[],
    }
}

fn literal(value: &str) -> HookWord<'_> {
    HookWord {
        value,
        kind: InvocationWordKind::Literal,
    }
}

#[test]
fn a_panicking_hook_abstains_and_is_quarantined_with_a_record() {
    let invocations = Rc::new(Cell::new(0));
    let host = host(&invocations);
    let installed = host.install_pack_hooks(PackPrograms {
        pack: "badlib".to_owned(),
        content_hash: "deadbeef".to_owned(),
        dsl_version: "1".to_owned(),
        programs: vec![HookProgram::new(
            "badlib::boom",
            HookFamily::ConstFold,
            "panic",
        )],
    });
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
    let installed = host.install_pack_hooks(PackPrograms::new("slowlib").with(HookProgram::new(
        "slowlib::spin",
        HookFamily::ConstFold,
        "burn",
    )));
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
    let installed = host.install_pack_hooks(PackPrograms::new("mylib").with(HookProgram::new(
        "mylib::wrong",
        HookFamily::ConstFold,
        "error",
    )));
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

/// A body that writes outside its activation raises and abstains, the same
/// way on the first call and the thousandth. Unconfined, `fold [incr
/// ::counter]` answers 1, 2, 3 … on one engine, so its answer would be the
/// call's position rather than a function of its arguments. A second hook in
/// the same pack, sharing the engine, finds nothing ever written. This runs
/// on the real VM, because the confinement is the engine's
/// (`Engine::confine_stores`); the hostile engine only claims it.
#[test]
fn a_body_with_a_global_counter_answers_identically_on_every_call() {
    let host = tcl_spec_hooks::tclvm_host();
    let installed = host.install_pack_hooks(
        PackPrograms::new("mylib")
            .with(HookProgram::new(
                "mylib::count",
                HookFamily::ConstFold,
                "fold [incr ::counter]",
            ))
            .with(HookProgram::new(
                "mylib::peek",
                HookFamily::ConstFold,
                "fold $::counter",
            )),
    );
    let count = installed[0].slot.expect("installed");
    let peek = installed[1].slot.expect("installed");
    let words = [literal("x")];
    for _ in 0..1000 {
        assert_eq!(host.invoke(count, &call(&words)), HookAnswer::Abstain);
    }
    assert!(!host.is_quarantined(count), "an error is not a crash");
    assert_eq!(
        host.invoke(peek, &call(&words)),
        HookAnswer::Abstain,
        "nothing was ever written"
    );
    let log = host.error_log();
    assert!(
        log.iter()
            .any(|line| line.contains("stores are confined to the activation")),
        "{log:?}"
    );
    assert!(
        log.iter().any(|line| line.contains("no such variable")),
        "{log:?}"
    );
}

/// The `rand()` generator is refused under every pinned release: its seed
/// is interpreter state one invocation would leave for the next, so a
/// `srand` in one hook and a `rand()` in another would fold a draw that
/// depends on the order the analysis happened to call them in. Under 8.4
/// and the releases derived from it (`f5-irules`) the functions are `expr`
/// builtins the whitelist cannot remove, and the confined VM refuses them;
/// from 8.5 they are commands the whitelist drops. The other math
/// functions stay: `abs(-1)` folds under every release.
#[test]
fn the_generator_is_refused_under_every_pinned_release() {
    let host = tcl_spec_hooks::tclvm_host();
    let installed = host.install_pack_hooks(
        PackPrograms::new("q")
            .with(
                HookProgram::new(
                    "q::seed",
                    HookFamily::ConstFold,
                    "expr {srand([lindex $words 0])}; fold seeded",
                )
                .pinned_to_release(),
            )
            .with(
                HookProgram::new("q::draw", HookFamily::ConstFold, "fold [expr {rand()}]")
                    .pinned_to_release(),
            )
            .with(
                HookProgram::new("q::abs", HookFamily::ConstFold, "fold [expr {abs(-1)}]")
                    .pinned_to_release(),
            ),
    );
    let [seed, draw, abs] = [0, 1, 2].map(|index| installed[index].slot.expect("installed"));
    for release in ["tcl8.4", "f5-irules", "tcl8.6", "tcl9.0"] {
        let words = [literal("7")];
        let pinned = HookCall {
            dialect: Some(release),
            ..call(&words)
        };
        assert_eq!(
            host.invoke(seed, &pinned),
            HookAnswer::Abstain,
            "{release}: srand"
        );
        for _ in 0..2 {
            assert_eq!(
                host.invoke(draw, &pinned),
                HookAnswer::Abstain,
                "{release}: rand"
            );
        }
        assert_eq!(
            host.invoke(abs, &pinned),
            HookAnswer::Fold("1".to_owned()),
            "{release}: abs"
        );
    }
}

/// The confinement leaves a body's own locals alone: a local accumulator
/// answers `a b` on every call.
#[test]
fn a_local_accumulator_is_unaffected() {
    let host = tcl_spec_hooks::tclvm_host();
    let installed = host.install_pack_hooks(PackPrograms::new("mylib").with(HookProgram::new(
        "mylib::acc",
        HookFamily::ConstFold,
        "set acc {}; foreach x {a b} {lappend acc $x}; fold $acc",
    )));
    let slot = installed[0].slot.expect("installed");
    let words = [literal("x")];
    for _ in 0..3 {
        assert_eq!(
            host.invoke(slot, &call(&words)),
            HookAnswer::Fold("a b".to_owned())
        );
    }
}

#[test]
fn a_crash_in_one_pack_leaves_another_pack_working() {
    let invocations = Rc::new(Cell::new(0));
    let host = host(&invocations);
    let bad = host.install_pack_hooks(PackPrograms::new("badlib").with(HookProgram::new(
        "badlib::boom",
        HookFamily::ConstFold,
        "panic",
    )));
    let good = host.install_pack_hooks(PackPrograms::new("goodlib").with(HookProgram::new(
        "goodlib::ok",
        HookFamily::ConstFold,
        "fold",
    )));
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
    let installed = host.install_pack_hooks(
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
    let installed = host.install_pack_hooks(PackPrograms::new("mylib").with(HookProgram::new(
        "mylib::fold",
        HookFamily::ConstFold,
        "fold",
    )));
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
    let orphan = pack_hooks::allocate(
        HookFamily::ConstFold,
        &pack_hooks::HookInputs::unrestricted(),
    )
    .expect("a slot");
    let words = [literal("x")];
    assert_eq!(host.invoke(orphan, &call(&words)), HookAnswer::Abstain);
}

/// A call's declared budget narrows the host's for that call alone
/// (`value-evaluation.md` § *The three nested budgets*): a body that
/// dispatches three hundred commands overruns a declared `-commands 100` and is
/// quarantined like any other budget blowout, while the same body in a
/// second hook on the same engine, called under the host's own budget
/// afterwards, answers — the narrower budget did not outlive its call.
#[test]
fn a_declared_budget_narrows_the_host_for_its_call_only() {
    let host = tcl_spec_hooks::tclvm_host();
    // `format` is dispatched as a command, never inlined as bytecode, so
    // each iteration spends one from the command budget.
    let body = "for {set i 0} {$i < 300} {incr i} {set n [format %d $i]}; fold $i";
    let installed = host.install_pack_hooks(
        PackPrograms::new("mylib")
            .with(HookProgram::new(
                "mylib::narrow",
                HookFamily::ConstFold,
                body,
            ))
            .with(HookProgram::new("mylib::wide", HookFamily::ConstFold, body)),
    );
    let narrow = installed[0].slot.expect("installed");
    let wide = installed[1].slot.expect("installed");
    let words = [literal("x")];
    let narrowed = HookCall {
        budget: tcl_registry::value_transfer::ImplementationBudget {
            commands: Some(100),
            ..tcl_registry::value_transfer::ImplementationBudget::default()
        },
        ..call(&words)
    };
    assert_eq!(host.invoke(narrow, &narrowed), HookAnswer::Abstain);
    assert!(host.is_quarantined(narrow), "an overrun quarantines");
    assert!(!host.is_available(narrow));
    let crashes = host.crash_records();
    assert_eq!(crashes.len(), 1, "{crashes:?}");
    assert_eq!(crashes[0].kind, CrashKind::CommandBudget);
    assert_eq!(
        host.invoke(wide, &call(&words)),
        HookAnswer::Fold("300".to_owned())
    );
    assert!(host.is_available(wide));
}

/// A declared budget above the host's runs under the host's: the host caps
/// each field a call declares at its own configuration, whatever that is,
/// and it is the only place the rule lives (the review of slice 4) — the
/// loader records `budget {-commands 900000}` as written. Here the host is
/// configured at two hundred commands, below the default, so a check
/// against the default host would have let the declaration through: the
/// body's three hundred commands overrun the host's two hundred and it is
/// quarantined like any other budget blowout, while a body that fits the
/// host's answers under the same declaration.
#[test]
fn a_declared_budget_above_the_hosts_runs_under_the_hosts() {
    let host = tcl_spec_hooks::tclvm_host_with(tcl_spec_hooks::HostConfig {
        budget: Budget::of_commands(200),
        ..tcl_spec_hooks::HostConfig::default()
    });
    let spin = |count: u32| {
        format!("for {{set i 0}} {{$i < {count}}} {{incr i}} {{set n [format %d $i]}}; fold $i")
    };
    let installed = host.install_pack_hooks(
        PackPrograms::new("mylib")
            .with(HookProgram::new(
                "mylib::long",
                HookFamily::ConstFold,
                spin(300),
            ))
            .with(HookProgram::new(
                "mylib::short",
                HookFamily::ConstFold,
                spin(50),
            )),
    );
    let long = installed[0].slot.expect("installed");
    let short = installed[1].slot.expect("installed");
    let words = [literal("x")];
    let widened = HookCall {
        budget: tcl_registry::value_transfer::ImplementationBudget {
            commands: Some(900_000),
            ..tcl_registry::value_transfer::ImplementationBudget::default()
        },
        ..call(&words)
    };
    assert_eq!(host.invoke(long, &widened), HookAnswer::Abstain);
    let crashes = host.crash_records();
    assert_eq!(crashes.len(), 1, "{crashes:?}");
    assert_eq!(crashes[0].kind, CrashKind::CommandBudget);
    assert!(host.is_quarantined(long), "an overrun quarantines");
    assert_eq!(
        host.invoke(short, &widened),
        HookAnswer::Fold("50".to_owned())
    );
}
