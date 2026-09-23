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

//! The hook host: per-pack engines, the calling conventions, and containment.

use std::cell::RefCell;
use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

use tcl_engine_api::{Budget, BudgetKind, CompileUnit, Engine, EngineError, Value};
use tcl_registry::invocation_words::InvocationWordKind;
use tcl_registry::pack_hooks::{self, HookAnswer, HookCall, HookFamily, HookSlot, PackHookHost};

use crate::crash::{CrashKind, CrashRecord};
use crate::emit::{Reading, Sink, answer_of, verbs_for};
use crate::program::{HookInstallation, HookProgram, PackPrograms};
use crate::sandbox::{SANDBOX_COMMANDS, builtins};

/// How the host runs hooks: what a body may spend, and who to blame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostConfig {
    /// The budget every invocation runs under.
    pub budget: Budget,
    /// The server build, recorded in a crash record.
    pub server_version: String,
}

impl Default for HostConfig {
    fn default() -> Self {
        Self {
            // Generous next to the measured 28 µs / 16.6 µs bodies, and far
            // below anything a user would notice as a hang: the budget exists
            // to bound a runaway, not to tune a fast hook.
            budget: Budget::of_commands(100_000)
                .with_wall_clock(std::time::Duration::from_millis(250))
                // 16 MiB: orders of magnitude above anything a spec hook has
                // any business building (the measured bodies produce a role
                // list or a folded literal), and far below the point where the
                // allocation itself is the problem. It exists to turn an OOM
                // abort — which takes the server, not the hook — into an
                // ordinary budget refusal the host can report and quarantine.
                .with_max_value_bytes(16 * 1024 * 1024),
            server_version: env!("CARGO_PKG_VERSION").to_owned(),
        }
    }
}

struct HookRuntime<H> {
    program: HookProgram,
    handle: H,
    /// Quarantined after its first crash or budget blowout: the command falls
    /// back to its declarative facts, so one bad hook cannot re-crash on every
    /// keystroke.
    quarantined: bool,
    /// Errors are logged once per hook per pack load, never per call site.
    errors_seen: u64,
}

struct PackRuntime<E: Engine> {
    pack: String,
    content_hash: String,
    dsl_version: String,
    engine: E,
    sink: Rc<Sink>,
    /// The `constraints` family's per-invocation reading view, refilled
    /// beside `sink` before every dispatch.
    reading: Rc<Reading>,
    /// Every family the pack's hooks belong to — the verbs a pinned engine
    /// is given, as the unpinned one was.
    families: Vec<HookFamily>,
    hooks: Vec<HookRuntime<E::Handle>>,
    /// The engines pinned to one release each, built on the first call a
    /// release-pinned hook receives under that release.
    pinned: Vec<PinnedEngine<E>>,
    /// Set when a hook of this pack panicked. A caught panic leaves the
    /// engine's own state unproven, so the whole pack stops answering — which
    /// is exactly the documented blast radius: "one library's bad hook costs
    /// that library its hooks, nothing else".
    poisoned: bool,
}

/// One pack's engine pinned to one release, with its own compilation of
/// every release-pinned hook.
struct PinnedEngine<E: Engine> {
    /// The profile the engine is pinned to, by canonical name.
    profile: String,
    /// `None` when the engine could not be pinned to the profile or its
    /// sandbox could not be built: every release-pinned hook of the pack
    /// abstains under that profile.
    engine: Option<E>,
    /// Each hook's handle on this engine, by hook index; `None` for a hook
    /// that is not release-pinned or did not compile here.
    handles: Vec<Option<E::Handle>>,
}

/// The `SpecTcl` hook host.
///
/// Generic over the engine, never over an engine's *identity*: the host names
/// no concrete engine anywhere below this signature, so a new engine slots in
/// under an unchanged host.
pub struct HookHost<E: Engine> {
    engine_factory: Box<dyn Fn() -> E>,
    config: HostConfig,
    packs: RefCell<Vec<PackRuntime<E>>>,
    slots: RefCell<HashMap<HookSlot, (usize, usize)>>,
    crashes: RefCell<Vec<CrashRecord>>,
    /// One line per hook that errored, kept so "logged once per hook per pack
    /// load" is a fact rather than an intention.
    error_log: RefCell<Vec<String>>,
}

impl<E: Engine> HookHost<E> {
    /// A host that builds one engine per pack with `engine_factory`.
    #[must_use]
    pub fn new(engine_factory: impl Fn() -> E + 'static) -> Self {
        Self::with_config(engine_factory, HostConfig::default())
    }

    /// A host with a non-default budget or server version.
    #[must_use]
    pub fn with_config(engine_factory: impl Fn() -> E + 'static, config: HostConfig) -> Self {
        Self {
            engine_factory: Box::new(engine_factory),
            config,
            packs: RefCell::new(Vec::new()),
            slots: RefCell::new(HashMap::new()),
            crashes: RefCell::new(Vec::new()),
            error_log: RefCell::new(Vec::new()),
        }
    }

    /// Compile one pack's hook bodies into their own engine and allocate a
    /// registry slot for each, returning what was installed.
    ///
    /// Per-pack isolation is structural: the engine is built here and never
    /// shared, so no interpreter state crosses packs. A release-pinned hook is
    /// compiled here too, so a body that does not compile is reported now;
    /// it runs only on the engines pinned to a release, built on first use.
    #[must_use]
    pub fn install_pack_hooks(&self, programs: PackPrograms) -> Vec<HookInstallation> {
        let sink = Rc::new(Sink::default());
        let reading = Rc::new(Reading::default());
        let mut families: Vec<HookFamily> = programs
            .programs
            .iter()
            .map(|program| program.family)
            .collect();
        families.sort_unstable();
        families.dedup();
        let (mut engine, failures) = self.sandboxed_engine(&families, &sink, &reading, None);

        let pack_index = self.packs.borrow().len();
        let mut runtimes = Vec::with_capacity(programs.programs.len());
        let mut installations = Vec::with_capacity(programs.programs.len());
        for program in programs.programs {
            let installation = self.compile_one(
                &mut engine,
                &program,
                pack_index,
                runtimes.len(),
                failures.first(),
            );
            if let Some(handle) = installation.1 {
                runtimes.push(HookRuntime {
                    program,
                    handle,
                    quarantined: false,
                    errors_seen: 0,
                });
            }
            installations.push(installation.0);
        }

        self.packs.borrow_mut().push(PackRuntime {
            pack: programs.pack,
            content_hash: programs.content_hash,
            dsl_version: programs.dsl_version,
            engine,
            sink,
            reading,
            families,
            hooks: runtimes,
            pinned: Vec::new(),
            poisoned: false,
        });
        installations
    }

    /// A fresh engine for one pack, pinned to `release` when one is named:
    /// the builtins, the families' verbs, the whitelist and the budget. What
    /// could not be set up is returned, so the caller declines every hook
    /// rather than run one in a wider sandbox than it was written for.
    fn sandboxed_engine(
        &self,
        families: &[HookFamily],
        sink: &Rc<Sink>,
        reading: &Rc<Reading>,
        release: Option<&str>,
    ) -> (E, Vec<String>) {
        let mut engine = (self.engine_factory)();
        let mut failures: Vec<String> = Vec::new();
        // Pinned first, as the engine contract asks: after the engine is
        // built and before anything is compiled on it.
        if let Some(profile) = release
            && let Err(error) = engine.set_release(profile)
        {
            failures.push(format!("release {profile}: {error}"));
        }
        for (name, command) in builtins() {
            if let Err(error) = engine.define_command(name, command) {
                failures.push(format!("{name}: {error}"));
            }
        }
        for &family in families {
            for (name, command) in verbs_for(family, sink, reading) {
                if let Err(error) = engine.define_command(name, command) {
                    failures.push(format!("{name}: {error}"));
                }
            }
        }
        // Whitelist first, then compile: a body is compiled inside the sandbox
        // it will run in, never in a wider one.
        if let Err(error) = engine.restrict_commands(SANDBOX_COMMANDS) {
            failures.push(format!("sandbox: {error}"));
        }
        // Writes stay in the activation: a body cannot leave state behind
        // that its next call reads, so its answer is its arguments'.
        if let Err(error) = engine.confine_stores() {
            failures.push(format!("stores: {error}"));
        }
        if let Err(error) = engine.set_budget(self.config.budget) {
            failures.push(format!("budget: {error}"));
        }
        (engine, failures)
    }

    /// The index of `pack`'s engine pinned to `profile`, building it — and
    /// compiling every release-pinned hook on it — the first time the
    /// profile is asked for. A profile the engine cannot pin, or a sandbox
    /// that cannot be built, is logged once and leaves the entry without an
    /// engine.
    fn pinned_engine(&self, pack: &mut PackRuntime<E>, profile: &str) -> usize {
        if let Some(index) = pack
            .pinned
            .iter()
            .position(|pinned| pinned.profile == profile)
        {
            return index;
        }
        let (mut engine, failures) =
            self.sandboxed_engine(&pack.families, &pack.sink, &pack.reading, Some(profile));
        let mut handles = Vec::with_capacity(pack.hooks.len());
        let engine = if let Some(failure) = failures.first() {
            self.error_log.borrow_mut().push(format!(
                "{}: no engine pinned to {profile}: {failure}",
                pack.pack
            ));
            None
        } else {
            for hook in &pack.hooks {
                if !hook.program.release_pinned {
                    handles.push(None);
                    continue;
                }
                let parameters: Vec<&str> = hook.program.effective_parameters();
                let label = hook.program.label();
                let unit = CompileUnit {
                    name: &label,
                    parameters: &parameters,
                    body: &hook.program.body,
                };
                match engine.compile(unit) {
                    Ok(handle) => handles.push(Some(handle)),
                    Err(error) => {
                        self.error_log
                            .borrow_mut()
                            .push(format!("{label} under {profile}: {error}"));
                        handles.push(None);
                    }
                }
            }
            Some(engine)
        };
        pack.pinned.push(PinnedEngine {
            profile: profile.to_owned(),
            engine,
            handles,
        });
        pack.pinned.len() - 1
    }

    /// Compile one hook body and claim its slot. Returns what to report and,
    /// when it worked, the handle to keep.
    fn compile_one(
        &self,
        engine: &mut E,
        program: &HookProgram,
        pack_index: usize,
        hook_index: usize,
        setup_failure: Option<&String>,
    ) -> (HookInstallation, Option<E::Handle>) {
        let declined = |reason: String| {
            (
                HookInstallation {
                    command: program.command.clone(),
                    owner: program.owner.clone(),
                    family: program.family,
                    slot: None,
                    declined: Some(reason),
                },
                None,
            )
        };
        if let Some(failure) = setup_failure {
            return declined(format!("the pack's sandbox could not be built: {failure}"));
        }
        // Derived from the declaration, not the conventional list: a hook
        // that did not declare `words` is not given them (see
        // `HookProgram::effective_parameters`).
        let parameters: Vec<&str> = program.effective_parameters();
        let label = program.label();
        let unit = CompileUnit {
            name: &label,
            parameters: &parameters,
            body: &program.body,
        };
        let handle = match engine.compile(unit) {
            Ok(handle) => handle,
            Err(error) => return declined(error.to_string()),
        };
        // A pre-assigned slot is bound, never re-allocated: the spec the
        // caller already built carries that slot's thunk, and every thread's
        // host must answer for the same one.
        let slot = if let Some(slot) = program.slot {
            slot
        } else if let Some(slot) = pack_hooks::allocate(program.family, &program.inputs) {
            slot
        } else {
            return declined(format!(
                "no {:?} slot is left in this process",
                program.family
            ));
        };
        self.slots
            .borrow_mut()
            .insert(slot, (pack_index, hook_index));
        (
            HookInstallation {
                command: program.command.clone(),
                owner: program.owner.clone(),
                family: program.family,
                slot: Some(slot),
                declined: None,
            },
            Some(handle),
        )
    }

    /// Every crash recorded so far, newest last.
    #[must_use]
    pub fn crash_records(&self) -> Vec<CrashRecord> {
        self.crashes.borrow().clone()
    }

    /// Every hook error logged so far — one line per hook, not per call site.
    #[must_use]
    pub fn error_log(&self) -> Vec<String> {
        self.error_log.borrow().clone()
    }

    /// Whether this slot's hook has been quarantined.
    #[must_use]
    pub fn is_quarantined(&self, slot: HookSlot) -> bool {
        let slots = self.slots.borrow();
        let Some(&(pack_index, hook_index)) = slots.get(&slot) else {
            return true;
        };
        let packs = self.packs.borrow();
        packs[pack_index].poisoned || packs[pack_index].hooks[hook_index].quarantined
    }

    fn record_crash(
        &self,
        pack_index: usize,
        hook_index: usize,
        kind: CrashKind,
        detail: String,
        shapes: Vec<InvocationWordKind>,
    ) {
        let mut packs = self.packs.borrow_mut();
        let pack = &mut packs[pack_index];
        let hook = &mut pack.hooks[hook_index];
        hook.quarantined = true;
        if kind == CrashKind::Panic {
            pack.poisoned = true;
        }
        self.crashes.borrow_mut().push(CrashRecord {
            pack: pack.pack.clone(),
            content_hash: pack.content_hash.clone(),
            hook: hook.program.label(),
            family: hook.program.family,
            dsl_version: pack.dsl_version.clone(),
            server_version: self.config.server_version.clone(),
            word_shapes: shapes,
            kind,
            detail,
        });
        // A quarantined hook's cached answers must not outlive it.
        pack_hooks::clear_cache();
    }
}

/// The DSL's `words`: the value where it is literal, the empty string where it
/// is not.
fn words_value(call: &HookCall<'_>) -> Value {
    Value::list(call.words.iter().map(|word| {
        if word.kind == InvocationWordKind::Literal {
            Value::string(word.value)
        } else {
            Value::Empty
        }
    }))
}

/// The DSL's `ctx` dict, exactly the keys
/// `docs/design/spec-dsl-examples/README.md` lists as always present, plus the
/// option-arity family's three.
fn ctx_value(program: &HookProgram, call: &HookCall<'_>) -> Value {
    let mut entries: Vec<(Value, Value)> = vec![
        (Value::string("command"), Value::string(&program.command)),
        (
            Value::string("subcommand"),
            program
                .owner
                .subcommand()
                .map_or(Value::Empty, Value::string),
        ),
        (Value::string("nwords"), Value::from(call.words.len())),
        (
            Value::string("kinds"),
            Value::list(
                call.words
                    .iter()
                    .map(|word| Value::string(pack_hooks::kind_word(word.kind))),
            ),
        ),
        (
            Value::string("tcl-version"),
            call.version.map_or(Value::Empty, |version| {
                Value::string(version.version_string())
            }),
        ),
        (
            Value::string("dialect"),
            call.dialect.map_or(Value::Empty, Value::string),
        ),
        (
            Value::string("in-event-body"),
            Value::from(call.in_event_body),
        ),
    ];
    if let Some(view) = call.constraints {
        entries.push((
            Value::string("options"),
            Value::dict(view.options.iter().map(|(name, value)| {
                (
                    Value::string(*name),
                    value.map_or(Value::Empty, Value::string),
                )
            })),
        ));
        entries.push((
            Value::string("positionals"),
            Value::list(
                view.positionals
                    .iter()
                    .map(|word| word.map_or(Value::Empty, Value::string)),
            ),
        ));
        // The abstention key: `complete` false means a `{*}` expansion or a
        // substituted word could be supplying anything, so nothing may be
        // proven absent and the body should `abstain`.
        entries.push((Value::string("complete"), Value::from(view.complete)));
    }
    if let Some(option) = call.option {
        entries.push((Value::string("option"), Value::string(option.option)));
        entries.push((Value::string("option-index"), Value::from(option.index)));
        entries.push((
            Value::string("option-value-start"),
            Value::from(option.value_start),
        ));
    }
    Value::dict(entries)
}

// The `dialect` ctx key comes from `HookCall::dialect`, which the registry
// fills from the ambient `DialectScope` the analyser enters, rather than
// being derived from the call's `TclVersion` — which can only ever spell a
// release, so an iRules document would tell a hook `tcl9.0` and no hook
// could distinguish a dialect from a version.

impl<E: Engine> HookHost<E> {
    /// Run hook `hook_index` of `pack` on `call` — on the pack's own engine,
    /// or on the engine pinned to the call's release when the hook runs
    /// pinned. `None` when the hook may not run at all: the pack is
    /// poisoned, the hook is quarantined, the call breaks the family's
    /// literal-only precondition, or a release-pinned hook has no release
    /// to run under or no engine that pins it.
    fn run(
        &self,
        pack: &mut PackRuntime<E>,
        hook_index: usize,
        call: &HookCall<'_>,
    ) -> Option<Outcome> {
        if pack.poisoned {
            return None;
        }
        let program = &pack.hooks[hook_index].program;
        if pack.hooks[hook_index].quarantined {
            return None;
        }
        // Normative precondition, enforced here as well as at the call
        // site: a fold body never runs on a call carrying a dynamic word.
        if program.family.requires_all_literal() && !call.all_literal() {
            return None;
        }
        // Matches `effective_parameters`: a hook that declared inputs
        // without `words` is compiled without that parameter, so passing
        // the value would be an arity error on every call.
        let arguments: Vec<Value> = if program.inputs.binds_words() {
            vec![words_value(call), ctx_value(program, call)]
        } else {
            vec![ctx_value(program, call)]
        };
        let family = program.family;
        // A release-pinned hook never runs at an engine's default release:
        // with no release named for the call it abstains.
        let pinned = if program.release_pinned {
            Some(self.pinned_engine(pack, call.dialect?))
        } else {
            None
        };
        let PackRuntime {
            engine,
            sink,
            reading,
            hooks,
            pinned: pinned_engines,
            ..
        } = pack;
        let hook = &mut hooks[hook_index];
        let (engine, handle) = match pinned {
            None => (engine, &hook.handle),
            Some(index) => {
                let PinnedEngine {
                    engine, handles, ..
                } = &mut pinned_engines[index];
                (engine.as_mut()?, handles.get(hook_index)?.as_ref()?)
            }
        };
        sink.clear();
        // The `constraints` family's reading verbs answer from this cell,
        // refilled per call exactly as the sink is cleared per call.
        match call.constraints {
            Some(view) => reading.set(view.options, view.positionals),
            None => reading.clear(),
        }
        let invoked = catch_unwind(AssertUnwindSafe(|| engine.invoke(handle, &arguments)));
        reading.clear();
        Some(match invoked {
            Ok(Ok(_)) => Outcome::Answer(answer_of(family, sink.drain())),
            Ok(Err(error)) => {
                sink.clear();
                match error {
                    EngineError::BudgetExceeded(BudgetKind::Commands) => {
                        Outcome::Crash(CrashKind::CommandBudget, error.to_string())
                    }
                    EngineError::BudgetExceeded(BudgetKind::WallClock) => {
                        Outcome::Crash(CrashKind::WallClockBudget, error.to_string())
                    }
                    other => {
                        hook.errors_seen += 1;
                        Outcome::Errored(hook.errors_seen, other.to_string())
                    }
                }
            }
            Err(payload) => {
                sink.clear();
                Outcome::Crash(CrashKind::Panic, panic_payload(payload.as_ref()))
            }
        })
    }
}

impl<E: Engine> PackHookHost for HookHost<E> {
    fn invoke(&self, slot: HookSlot, call: &HookCall<'_>) -> HookAnswer {
        let Some(&(pack_index, hook_index)) = self.slots.borrow().get(&slot) else {
            return HookAnswer::Abstain;
        };
        let outcome = {
            let mut packs = self.packs.borrow_mut();
            let Some(outcome) = self.run(&mut packs[pack_index], hook_index, call) else {
                return HookAnswer::Abstain;
            };
            outcome
        };
        match outcome {
            Outcome::Answer(answer) => answer,
            Outcome::Errored(seen, message) => {
                if seen == 1 {
                    let label = {
                        let packs = self.packs.borrow();
                        packs[pack_index].hooks[hook_index].program.label()
                    };
                    self.error_log
                        .borrow_mut()
                        .push(format!("{label}: {message}"));
                }
                HookAnswer::Abstain
            }
            Outcome::Crash(kind, detail) => {
                self.record_crash(
                    pack_index,
                    hook_index,
                    kind,
                    detail,
                    CrashRecord::shapes_of(call),
                );
                HookAnswer::Abstain
            }
        }
    }
}

enum Outcome {
    Answer(HookAnswer),
    Errored(u64, String),
    Crash(CrashKind, String),
}

fn panic_payload(payload: &(dyn std::any::Any + Send)) -> String {
    payload.downcast_ref::<&str>().map_or_else(
        || {
            payload
                .downcast_ref::<String>()
                .cloned()
                .unwrap_or_else(|| "panic with an unreadable payload".to_owned())
        },
        |message| (*message).to_owned(),
    )
}
