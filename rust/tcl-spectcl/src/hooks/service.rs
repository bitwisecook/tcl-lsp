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

//! **One warm hook VM per process.**
//!
//! A hook body runs on a `tcl-vm` engine, which is not `Send`, so the host
//! that owns the engines lives on one dedicated thread. It is built once per
//! published plan — every pack's sandbox bootstrapped and every body bound
//! before the first call — and serves every worker thread. A worker installs
//! a [`Proxy`], which costs nothing to build, so no analysis thread ever pays
//! a VM bootstrap, however often the pool turns its threads over.
//!
//! Per-pack isolation is unchanged: each pack still runs in its own sandboxed
//! engine inside that host, with its own budgets and quarantine.
//!
//! At install the service also **tabulates** each resolver whose body reads
//! its call only through `[llength $words]`: the answer for every word count
//! up to [`TABULATED_WORDS`] is computed once and published to the registry
//! (`pack_hooks::tabulate`), which then answers those calls without a host.

use std::collections::BTreeSet;
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::{Arc, Mutex, PoisonError};

use tcl_registry::InvocationWordKind;
use tcl_registry::pack_hooks::{
    ConstraintCallCtx, HookAnswer, HookCall, HookFamily, HookSlot, HookWord, OptionCallCtx,
    PackHookHost,
};
use tcl_registry::value_transfer::{ContextDependency, ImplementationBudget};

use super::HookPlan;

/// The largest word count a tabulated resolver is answered for.
pub(crate) const TABULATED_WORDS: usize = 32;

/// A constraint context, owned: its options, its positionals, and whether
/// the invocation was read to its end.
type OwnedConstraints = (
    Vec<(&'static str, Option<String>)>,
    Vec<Option<String>>,
    bool,
);

/// A hook call with every borrowed field owned, to cross to the service.
struct OwnedCall {
    words: Vec<(String, InvocationWordKind)>,
    version: Option<tcl_dialect::TclVersion>,
    in_event_body: bool,
    option: Option<(String, usize, usize)>,
    constraints: Option<OwnedConstraints>,
    dialect: Option<&'static str>,
    targets: Vec<usize>,
    budget: ImplementationBudget,
    depends: Vec<ContextDependency>,
}

impl OwnedCall {
    fn of(call: &HookCall<'_>) -> Self {
        Self {
            words: call
                .words
                .iter()
                .map(|word| (word.value.to_owned(), word.kind))
                .collect(),
            version: call.version,
            in_event_body: call.in_event_body,
            option: call
                .option
                .map(|option| (option.option.to_owned(), option.index, option.value_start)),
            constraints: call.constraints.map(|constraints| {
                (
                    constraints
                        .options
                        .iter()
                        .map(|&(name, value)| (name, value.map(str::to_owned)))
                        .collect(),
                    constraints
                        .positionals
                        .iter()
                        .map(|value| value.map(str::to_owned))
                        .collect(),
                    constraints.complete,
                )
            }),
            dialect: call.dialect,
            targets: call.targets.to_vec(),
            budget: call.budget,
            depends: call.depends.to_vec(),
        }
    }

    fn with<R>(&self, run: impl FnOnce(&HookCall<'_>) -> R) -> R {
        let words: Vec<HookWord<'_>> = self
            .words
            .iter()
            .map(|(value, kind)| HookWord { value, kind: *kind })
            .collect();
        let options: Vec<(&'static str, Option<&str>)> = self
            .constraints
            .iter()
            .flat_map(|(options, _, _)| {
                options
                    .iter()
                    .map(|(name, value)| (*name, value.as_deref()))
            })
            .collect();
        let positionals: Vec<Option<&str>> = self
            .constraints
            .iter()
            .flat_map(|(_, positionals, _)| positionals.iter().map(Option::as_deref))
            .collect();
        let call = HookCall {
            words: &words,
            version: self.version,
            in_event_body: self.in_event_body,
            option: self
                .option
                .as_ref()
                .map(|(option, index, value_start)| OptionCallCtx {
                    option,
                    index: *index,
                    value_start: *value_start,
                }),
            constraints: self
                .constraints
                .as_ref()
                .map(|(_, _, complete)| ConstraintCallCtx {
                    options: &options,
                    positionals: &positionals,
                    complete: *complete,
                }),
            dialect: self.dialect,
            targets: &self.targets,
            budget: self.budget,
            depends: &self.depends,
        };
        run(&call)
    }
}

enum Message {
    /// Serve a plan. `eager` builds its host now; a plan whose every body is
    /// already tabulated builds it only if a call falls outside a table.
    Install { plan: Arc<HookPlan>, eager: bool },
    Invoke {
        slot: HookSlot,
        call: Box<OwnedCall>,
        reply: mpsc::SyncSender<HookAnswer>,
    },
}

/// The service's inbox, and the plan it serves — kept so a service that had
/// to be restarted comes back serving the same plan.
struct Service {
    sender: Option<mpsc::Sender<Message>>,
    plan: Option<(Arc<HookPlan>, bool)>,
}

static SERVICE: Mutex<Service> = Mutex::new(Service {
    sender: None,
    plan: None,
});

/// Slots whose hook the service quarantined, process-wide.
static QUARANTINED: Mutex<BTreeSet<HookSlot>> = Mutex::new(BTreeSet::new());

fn spawn() -> mpsc::Sender<Message> {
    let (sender, inbox) = mpsc::channel();
    std::thread::Builder::new()
        .name("spectcl-hooks".to_owned())
        // A hook body is bounded by its budgets, not by this stack; the
        // headroom is for the VM's own recursion during a deep body.
        .stack_size(32 * 1024 * 1024)
        .spawn(move || serve(&inbox))
        .expect("spawn the SpecTcl hook service");
    sender
}

/// Send `message`, starting (or restarting) the service as needed.
fn send(message: Message) -> bool {
    let mut service = SERVICE.lock().unwrap_or_else(PoisonError::into_inner);
    let sender = if let Some(sender) = &service.sender {
        sender.clone()
    } else {
        let sender = spawn();
        if let Some((plan, eager)) = &service.plan {
            let _ = sender.send(Message::Install {
                plan: Arc::clone(plan),
                eager: *eager,
            });
        }
        service.sender = Some(sender.clone());
        sender
    };
    if let Err(mpsc::SendError(message)) = sender.send(message) {
        // The service died; the next send starts a fresh one on the same plan.
        service.sender = None;
        drop(service);
        if let Message::Invoke { .. } = message {
            return false;
        }
    }
    true
}

/// Serve `plan` from now on: build its host on the service thread, bind
/// every body, and tabulate what can be. `precompiled` are the slots already
/// answered from a shipped table; when they are every body the plan has, the
/// host is built only if a call falls outside a table.
pub(crate) fn install(plan: Arc<HookPlan>, precompiled: &BTreeSet<HookSlot>) {
    QUARANTINED
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clear();
    let eager = plan.packs().iter().any(|programs| {
        programs
            .programs
            .iter()
            .any(|program| program.slot.is_none_or(|slot| !precompiled.contains(&slot)))
    });
    SERVICE.lock().unwrap_or_else(PoisonError::into_inner).plan = Some((Arc::clone(&plan), eager));
    let _ = send(Message::Install { plan, eager });
}

/// A worker thread's host: every call goes to the service.
pub(crate) struct Proxy;

impl PackHookHost for Proxy {
    fn invoke(&self, slot: HookSlot, call: &HookCall<'_>) -> HookAnswer {
        let (reply, answer) = mpsc::sync_channel(1);
        let message = Message::Invoke {
            slot,
            call: Box::new(OwnedCall::of(call)),
            reply,
        };
        if !send(message) {
            return HookAnswer::Abstain;
        }
        answer.recv().unwrap_or(HookAnswer::Abstain)
    }

    fn is_available(&self, slot: HookSlot) -> bool {
        !QUARANTINED
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .contains(&slot)
    }
}

/// How many hosts this process has built for the service or for
/// precomputation — each one a VM bootstrap per pack.
pub(crate) static HOSTS_BUILT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// The host for `plan`: one sandboxed engine per pack, every body bound.
pub(crate) fn build_host(plan: &HookPlan) -> Rc<dyn PackHookHost> {
    HOSTS_BUILT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let built = tcl_spec_hooks::tclvm_host();
    for programs in plan.packs() {
        let _installed = built.install_pack_hooks(programs.clone());
    }
    Rc::new(built)
}

fn serve(inbox: &mpsc::Receiver<Message>) {
    let mut host: Option<Rc<dyn PackHookHost>> = None;
    let mut deferred: Option<Arc<HookPlan>> = None;
    while let Ok(message) = inbox.recv() {
        match message {
            Message::Install { plan, eager } => {
                host = None;
                deferred = None;
                if plan.is_empty() {
                    continue;
                }
                if eager {
                    let built = build_host(&plan);
                    tabulate(built.as_ref(), &plan);
                    host = Some(built);
                } else {
                    deferred = Some(plan);
                }
            }
            Message::Invoke { slot, call, reply } => {
                if host.is_none()
                    && let Some(plan) = deferred.take()
                {
                    host = Some(build_host(&plan));
                }
                let answer = match &host {
                    Some(host) => {
                        let answer = call.with(|call| host.invoke(slot, call));
                        if !host.is_available(slot) {
                            QUARANTINED
                                .lock()
                                .unwrap_or_else(PoisonError::into_inner)
                                .insert(slot);
                        }
                        answer
                    }
                    None => HookAnswer::Abstain,
                };
                let _ = reply.send(answer);
            }
        }
    }
}

/// The families whose answer is a function of the call's words alone, with
/// no option or constraint context.
const WORD_FAMILIES: &[HookFamily] = &[
    HookFamily::ArgRoleResolver,
    HookFamily::CommandPrefixResolver,
    HookFamily::ScriptTimingResolver,
];

/// Whether `body`, with `parameters`, reads its call only through the count
/// of its words: every mention of the words parameter is `[llength $words]`,
/// and the context parameter is never mentioned.
pub(crate) fn reads_only_word_count(parameters: &[String], body: &str) -> bool {
    let Some(words) = parameters.first() else {
        return true;
    };
    let mut rest = body.to_owned();
    for idiom in [
        format!("[llength ${words}]"),
        format!("[llength ${{{words}}}]"),
    ] {
        rest = rest.replace(&idiom, "");
    }
    let mentions = |name: &str| {
        rest.match_indices(name).any(|(at, _)| {
            let before = rest[..at].chars().next_back();
            let after = rest[at + name.len()..].chars().next();
            !before.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == ':')
                && !after.is_some_and(|c| c.is_alphanumeric() || c == '_')
        })
    };
    !mentions(words) && parameters.get(1).is_none_or(|ctx| !mentions(ctx))
}

/// Tabulate every count-only resolver of `plan` on `host` ([`table_for`]).
fn tabulate(host: &dyn PackHookHost, plan: &HookPlan) {
    for programs in plan.packs() {
        for program in &programs.programs {
            if let Some(slot) = program.slot
                && is_tabulable(program)
                && let Some(answers) = table_for(host, slot)
            {
                tcl_registry::pack_hooks::tabulate(slot, answers);
            }
        }
    }
}

/// The identity a precompiled table is keyed by: `xxh3` of the parameter
/// list and the body. A body is a pure function in a deterministic sandbox,
/// so its answers follow from its text alone.
#[must_use]
pub fn body_hash(program: &tcl_spec_hooks::HookProgram) -> u64 {
    let mut hasher = xxhash_rust::xxh3::Xxh3::new();
    for parameter in &program.parameters {
        hasher.update(parameter.as_bytes());
        hasher.update(&[0]);
    }
    hasher.update(&[1]);
    hasher.update(program.body.as_bytes());
    hasher.digest()
}

/// Whether `program` is a family answered from its words alone and its body
/// reads them only through their count.
pub(crate) fn is_tabulable(program: &tcl_spec_hooks::HookProgram) -> bool {
    WORD_FAMILIES.contains(&program.family)
        && reads_only_word_count(&program.parameters, &program.body)
}

/// `slot`'s answer at every word count up to [`TABULATED_WORDS`], when three
/// probes — empty literal words with no dialect, distinct literal words under
/// Tcl 9.0, and computed words under iRules — agree at every count.
pub(crate) fn table_for(host: &dyn PackHookHost, slot: HookSlot) -> Option<Vec<HookAnswer>> {
    let mut answers = Vec::with_capacity(TABULATED_WORDS + 1);
    for count in 0..=TABULATED_WORDS {
        let probe = |value: &dyn Fn(usize) -> String,
                     kind: InvocationWordKind,
                     dialect: Option<&'static str>| {
            let values: Vec<String> = (0..count).map(value).collect();
            let words: Vec<HookWord<'_>> = values
                .iter()
                .map(|value| HookWord { value, kind })
                .collect();
            host.invoke(
                slot,
                &HookCall {
                    words: &words,
                    version: None,
                    in_event_body: false,
                    option: None,
                    constraints: None,
                    dialect,
                    targets: &[],
                    budget: ImplementationBudget::default(),
                    depends: &[],
                },
            )
        };
        let first = probe(&|_| String::new(), InvocationWordKind::Literal, None);
        let second = probe(
            &|index| format!("-w{index}"),
            InvocationWordKind::Literal,
            Some("tcl9.0"),
        );
        let third = probe(
            &|_| String::new(),
            InvocationWordKind::Dynamic,
            Some("f5-irules"),
        );
        if first != second || second != third {
            return None;
        }
        answers.push(first);
    }
    host.is_available(slot).then_some(answers)
}

#[cfg(test)]
mod tests {
    use super::reads_only_word_count;

    fn params() -> Vec<String> {
        vec!["words".to_owned(), "ctx".to_owned()]
    }

    #[test]
    fn a_body_reading_only_the_word_count_is_recognised() {
        assert!(reads_only_word_count(
            &params(),
            "set n [llength $words]\nif {$n >= 2} { role [expr {$n - 1}] Body }"
        ));
        assert!(reads_only_word_count(
            &params(),
            "role [llength ${words}] Body"
        ));
    }

    #[test]
    fn any_other_read_of_the_words_or_the_context_is_not() {
        for body in [
            "role [lindex $words 0] Body",
            "set n [llength $words]; foreach w $words {}",
            "if {[dict get $ctx dialect] eq {x}} {role 0 Body}",
            "upvar 0 words w; role [llength $w] Body",
            "set v ctx; role 0 Body",
        ] {
            assert!(!reads_only_word_count(&params(), body), "{body}");
        }
        assert!(reads_only_word_count(
            &params(),
            "set wordsmith 1; role 0 Body"
        ));
    }
}
