// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The owned application closed consumer.
//!
//! One worker thread answers the selected pack-hook folds with an owned
//! evaluator, retires that evaluator, then admits the rewritten candidate and
//! runs it once in a private VM. Only detached bytes and a lineage receipt
//! leave the worker. The transcript shows that the restricted candidate ran;
//! the folds' authority is the ordinary O129 selection plus a quiet private
//! hook host, and no runtime-backed original is cross-checked.

use std::sync::Arc;

use tcl_compiler::optimiser::closed_consumer::{
    ApplicationRewritePlan, ApplicationText, MIN_CLOSED_STACK_BYTES, SelectedApplicationFold,
    select_application_folds,
};
use tcl_registry::pack_hooks;
use tcl_registry::{CommandRegistry, RegistrySemanticKey};
use tcl_runtime_api::closed_native_consumer::{
    ClosedNativeBackend, ClosedNativeConsumerOwner, ClosedNativeState, ClosedSourceOutput,
    ClosedSourceRefusal, ClosedSourceTranscript,
};
use tcl_spec_hooks::HostConfig;
use tcl_spec_hooks::private_application::PrivateApplicationVm;

use super::owned_evaluator::{OWNED_PROFILE, OwnedApplicationEvaluator, OwnedFoldLineage};
use crate::install::OwnedHookRegistry;
use crate::pack::PackSet;

/// What vouches for each replaced value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoldAuthority {
    /// The ordinary O129 fold gates selected the call and the pack hook
    /// answered on a private host that stayed quiet.
    O129GatesWithQuietHost,
}

/// Lineage of one owned execution, detached from every engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedApplicationReceipt {
    /// The private registry's semantic key.
    pub registry: RegistrySemanticKey,
    /// The installed program behind each replaced substitution, in order.
    pub folds: Vec<OwnedFoldLineage>,
    /// The authority for every replaced value.
    pub authority: FoldAuthority,
    /// Whether a runtime-backed original command was also checked; it never
    /// is, since only the candidate executes.
    pub runtime_cross_checked: bool,
}

/// The detached transcript and rewritten source, with their receipt.
#[derive(Debug)]
pub struct OwnedApplicationExecution {
    /// What the private VM wrote and the candidate it ran.
    pub output: ClosedSourceOutput,
    /// Where each replaced value came from.
    pub receipt: OwnedApplicationReceipt,
}

/// Answer the packs' hooks for `source` under a private `tcl9.1` registry,
/// rewrite the answered substitutions, and run the admitted candidate once in
/// a private VM on its own worker thread.
///
/// # Errors
/// Refuses an unavailable worker, profile, bootstrap or hook plan; the
/// private-store bound (`Budget`); unsupported or unanswered source; any
/// candidate outside the admitted inventory or returned-code closure; any
/// drift of the evaluator, registry, compiler or VM state; and any execution
/// other than a normal completion within its limits.
pub fn execute_owned_application_source(
    packs: &PackSet,
    source: String,
) -> Result<OwnedApplicationExecution, ClosedSourceRefusal> {
    let stack = usize::try_from(MIN_CLOSED_STACK_BYTES).map_err(|_| ClosedSourceRefusal::Budget)?;
    let packs = packs.clone();
    std::thread::Builder::new()
        .stack_size(stack)
        .name("owned-application-consumer".into())
        .spawn(move || owned_execution(&packs, &source))
        .map_err(|_| ClosedSourceRefusal::Execution)?
        .join()
        .map_err(|_| ClosedSourceRefusal::Execution)?
}

fn owned_profile() -> Result<&'static tcl_dialect::DialectProfile, ClosedSourceRefusal> {
    tcl_registry::model::ingress::resolve_known_environment(OWNED_PROFILE)
        .and_then(|environment| environment.catalogue_profile())
        .ok_or(ClosedSourceRefusal::NativeContext)
}

fn owned_execution(
    packs: &PackSet,
    source: &str,
) -> Result<OwnedApplicationExecution, ClosedSourceRefusal> {
    let profile = owned_profile()?;
    let installed = crate::install::bounded_owned_hook_registry(profile, packs)
        .ok_or(ClosedSourceRefusal::Budget)?;
    let registry = Arc::clone(installed.registry());
    let answered = answer_phase(installed, source, profile)?;
    execution_phase(&registry, profile, source, answered)
}

type Answers = Vec<(SelectedApplicationFold, String)>;

/// Phase one's result: the answers, their receipt and the exact store that
/// answered them. Content-equal stores are not interchangeable.
struct Answered {
    answers: Answers,
    receipt: OwnedApplicationReceipt,
    store: *const CommandRegistry,
}

/// Phase one: select, answer and record lineage, then retire the evaluator
/// and prove the thread holds no hook host.
fn answer_phase(
    installed: Arc<OwnedHookRegistry>,
    source: &str,
    profile: &'static tcl_dialect::DialectProfile,
) -> Result<Answered, ClosedSourceRefusal> {
    let owner = OwnedApplicationEvaluator::new(installed, HostConfig::default())?;
    let selected = select_application_folds(source, owner.registry(), profile)?;
    let mut answers = Vec::new();
    let mut folds = Vec::new();
    for fold in selected {
        if let Some((value, lineage)) = owner.answer(fold.request())? {
            answers.push((fold, value));
            folds.push(lineage);
        }
    }
    owner.check_current()?;
    let registry = owner.registry().snapshot().semantic_key();
    let store = std::ptr::from_ref(owner.registry());
    drop(owner);
    if pack_hooks::has_host() {
        return Err(ClosedSourceRefusal::NativeContext);
    }
    Ok(Answered {
        answers,
        receipt: OwnedApplicationReceipt {
            registry,
            folds,
            authority: FoldAuthority::O129GatesWithQuietHost,
            runtime_cross_checked: false,
        },
        store,
    })
}

/// Phase two: deny every evaluator on this thread, plan and restrict the
/// candidate, and drive the closed-consumer owner to one execution.
fn execution_phase(
    registry: &Arc<CommandRegistry>,
    profile: &'static tcl_dialect::DialectProfile,
    source: &str,
    answered: Answered,
) -> Result<OwnedApplicationExecution, ClosedSourceRefusal> {
    let backend = application_backend(registry, profile, source, &answered)?;
    let receipt = answered.receipt;
    let candidate = backend.plan.candidate().to_owned();
    let output = {
        let owner = ClosedNativeConsumerOwner::new(backend)?;
        let scope = owner.scope()?;
        let rewrite = scope.select(candidate)?;
        owner.execute(rewrite)?
    };
    Ok(OwnedApplicationExecution { output, receipt })
}

fn application_backend(
    registry: &Arc<CommandRegistry>,
    profile: &'static tcl_dialect::DialectProfile,
    source: &str,
    answered: &Answered,
) -> Result<ApplicationBackend, ClosedSourceRefusal> {
    if pack_hooks::has_host() || !pack_hooks::deny_external_evaluators_on_private_thread() {
        return Err(ClosedSourceRefusal::NativeContext);
    }
    if !std::ptr::eq(Arc::as_ptr(registry), answered.store)
        || registry.snapshot().semantic_key() != answered.receipt.registry
    {
        return Err(ClosedSourceRefusal::Currency);
    }
    let plan = ApplicationRewritePlan::new(source, registry, profile, &answered.answers)?;
    // The candidate's inventory restricts the VM before the owner exists.
    let vm = PrivateApplicationVm::new(
        registry,
        plan.registry_key(),
        plan.candidate(),
        plan.command_names().to_vec(),
        plan.scalar_names().to_vec(),
    )?;
    Ok(ApplicationBackend {
        plan,
        registry: Arc::clone(registry),
        vm,
    })
}

struct ApplicationBackend {
    plan: ApplicationRewritePlan,
    registry: Arc<CommandRegistry>,
    vm: PrivateApplicationVm,
}

impl ClosedNativeBackend for ApplicationBackend {
    type Currency = RegistrySemanticKey;

    fn original_source(&self) -> &str {
        self.plan.source()
    }

    fn capture_closed_state(
        &self,
    ) -> Result<ClosedNativeState<RegistrySemanticKey>, ClosedSourceRefusal> {
        let entry = self.vm.capture_state()?;
        if !pack_hooks::private_thread_evaluators_are_closed() {
            return Err(ClosedSourceRefusal::NativeContext);
        }
        let currency = self.registry.snapshot().semantic_key();
        if &currency != self.plan.registry_key() {
            return Err(ClosedSourceRefusal::Currency);
        }
        Ok(ClosedNativeState { entry, currency })
    }

    fn validate_rewrite(&self, candidate: &str) -> Result<(), ClosedSourceRefusal> {
        self.plan.accepts(candidate, &self.registry).map(|_| ())
    }

    fn execute_owned(
        self,
        candidate: String,
    ) -> Result<ClosedSourceTranscript, ClosedSourceRefusal> {
        // The original corresponds but cannot run: its pack commands have no
        // binding in the private VM.
        if self.plan.accepts(&candidate, &self.registry)? == ApplicationText::Original {
            return Err(ClosedSourceRefusal::Selection);
        }
        let prepared = self.vm.prepare(&candidate)?;
        self.vm.execute(prepared)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::{Origin, PackFile, Tier};

    fn hook_pack(name: &str, body: &str) -> PackSet {
        let dir = std::env::temp_dir().join(format!(
            "tcl-spectcl-owned-application-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("owner.tclspec");
        std::fs::write(
            &path,
            format!(
                "speclib owner 2.0 {{\n    command owner::f {{\n        arity 1\n        \
                 arg 0 -role Value\n        const_fold -inputs {{words}} {{words ctx}} {{\n{body}\n        }}\n    }}\n}}\n"
            ),
        )
        .unwrap();
        let packs = crate::pack::load(&[PackFile {
            tier: Tier::Workspace,
            path,
            origin: Origin::DotDir,
            dependency_tier: None,
        }]);
        let _ = std::fs::remove_dir_all(&dir);
        assert!(packs.notices.is_empty(), "{:?}", packs.notices);
        packs
    }

    fn length_pack() -> PackSet {
        hook_pack("length", "fold [string length [lindex $words 0]]")
    }

    fn executed(packs: &PackSet, source: &str) -> Result<(String, String), ClosedSourceRefusal> {
        let execution = execute_owned_application_source(packs, source.to_owned())?;
        assert_eq!(
            execution.receipt.authority,
            FoldAuthority::O129GatesWithQuietHost
        );
        assert!(!execution.receipt.runtime_cross_checked);
        Ok((
            String::from_utf8(execution.output.transcript.stdout).unwrap(),
            execution.output.rewritten_source,
        ))
    }

    #[test]
    fn an_answered_fold_runs_as_its_rewritten_candidate() {
        let packs = length_pack();
        assert_eq!(
            executed(&packs, "puts [owner::f abcde]\n").unwrap(),
            ("5\n".to_owned(), "puts 5\n".to_owned())
        );
        assert_eq!(
            executed(&packs, "set y [owner::f abc]\nputs $y\n")
                .unwrap()
                .0,
            "3\n"
        );
        assert_eq!(
            executed(&packs, "puts [owner::f ab]\nputs [owner::f abcd]\n")
                .unwrap()
                .0,
            "2\n4\n"
        );
    }

    #[test]
    fn the_receipt_names_each_installed_program() {
        let packs = length_pack();
        let execution = execute_owned_application_source(
            &packs,
            "puts [owner::f ab]\nputs [owner::f a]\n".into(),
        )
        .unwrap();
        assert_eq!(execution.receipt.folds.len(), 2);
        assert!(
            execution
                .receipt
                .folds
                .iter()
                .all(|fold| fold.pack == "owner" && fold.command == "owner::f")
        );
    }

    /// N17: `$arr($a:b)` reads element `x:b` in C Tcl.
    #[test]
    fn a_colon_suffixed_element_answer_runs_as_c_tcl_reads_it() {
        let packs = hook_pack(
            "colon",
            "set a x\nset a:b BAD\nset arr(x:b) good\nset arr(BAD) bad\nfold $arr($a:b)",
        );
        assert_eq!(executed(&packs, "puts [owner::f q]\n").unwrap().0, "good\n");
    }

    #[test]
    fn an_answer_with_spaces_braces_and_a_newline_renders_and_relexes() {
        let packs = hook_pack("render", "fold \"a \\{b\\nc\"");
        let (stdout, rewritten) = executed(&packs, "puts [owner::f q]\n").unwrap();
        assert_eq!(stdout, "a {b\nc\n");
        assert_ne!(rewritten, "puts [owner::f q]\n");
    }

    #[test]
    fn commands_outside_the_inventory_are_refused() {
        let packs = length_pack();
        for extra in [
            "catch {puts hi}",
            "eval {puts hi}",
            "rename puts p",
            "trace add variable y write t",
            "interp limit {} commands 1",
            "puts stderr hi",
            "coroutine c puts hi",
            "set c puts\n$c x",
        ] {
            assert_eq!(
                executed(&packs, &format!("puts [owner::f ab]\n{extra}\n")).err(),
                Some(ClosedSourceRefusal::Source),
                "{extra}"
            );
        }
    }

    #[test]
    fn substitutions_outside_whole_argument_words_are_not_rewritten() {
        let packs = length_pack();
        for source in [
            "puts {*}[owner::f ab]\n",
            "[owner::f ab] x\n",
            "puts \"[owner::f ab]\"\n",
        ] {
            assert_eq!(
                executed(&packs, source).err(),
                Some(ClosedSourceRefusal::Source),
                "{source}"
            );
        }
    }

    #[test]
    fn a_long_answer_breaches_the_output_bound() {
        let packs = hook_pack("long", "fold [string repeat x 300]");
        assert_eq!(
            executed(&packs, "puts [owner::f q]\n").err(),
            Some(ClosedSourceRefusal::Execution)
        );
    }

    #[test]
    fn a_quarantined_hook_retires_the_evaluator() {
        let packs = hook_pack("spin", "while 1 {}");
        assert_eq!(
            executed(&packs, "puts [owner::f q]\n").err(),
            Some(ClosedSourceRefusal::Currency)
        );
    }

    /// A hook that tries to write output or raises abstains, so nothing folds
    /// and the source is unsupported. The hook sandbox exposes no command
    /// that reaches the private host's output; a write that did would leave
    /// it non-quiet and retire the evaluator (`check_quiet_host`).
    #[test]
    fn a_hook_that_writes_or_errors_folds_nothing() {
        for (name, body) in [
            ("noisy", "puts noise\nfold 1"),
            ("noisy-channel", "puts stdout noise\nfold 1"),
            ("raising", "error boom"),
        ] {
            let packs = hook_pack(name, body);
            assert_eq!(
                executed(&packs, "puts [owner::f q]\n").err(),
                Some(ClosedSourceRefusal::Source),
                "{body}"
            );
        }
    }

    fn on_worker(body: impl FnOnce() + Send + 'static) {
        std::thread::Builder::new()
            .stack_size(usize::try_from(MIN_CLOSED_STACK_BYTES).unwrap())
            .spawn(body)
            .unwrap()
            .join()
            .unwrap();
    }

    fn answered(packs: &PackSet, source: &str) -> (Arc<CommandRegistry>, Answered) {
        let profile = owned_profile().unwrap();
        let installed = crate::install::bounded_owned_hook_registry(profile, packs).unwrap();
        let registry = Arc::clone(installed.registry());
        (registry, answer_phase(installed, source, profile).unwrap())
    }

    #[test]
    fn the_original_corresponds_but_is_never_executed() {
        let packs = length_pack();
        on_worker(move || {
            let source = "puts [owner::f abc]\n";
            let (registry, answered) = answered(&packs, source);
            let profile = owned_profile().unwrap();
            let backend = application_backend(&registry, profile, source, &answered).unwrap();
            assert_eq!(
                backend.plan.accepts("puts 4\n", &registry),
                Err(ClosedSourceRefusal::Source),
                "byte drift"
            );
            let owner = ClosedNativeConsumerOwner::new(backend).unwrap();
            let scope = owner.scope().unwrap();
            let original = scope.select(source.to_owned()).unwrap();
            assert_eq!(
                owner.execute(original).err(),
                Some(ClosedSourceRefusal::Selection)
            );
        });
    }

    /// A store built again from the same packs did not answer the folds; it
    /// cannot stand in for the one that did, whatever its key.
    #[test]
    fn a_content_equal_store_other_than_the_answering_one_is_refused() {
        let packs = length_pack();
        on_worker(move || {
            let source = "puts [owner::f abc]\n";
            let (registry, answered) = answered(&packs, source);
            let profile = owned_profile().unwrap();
            let other = Arc::clone(crate::install::owned_hook_registry(profile, &packs).registry());
            assert!(!Arc::ptr_eq(&registry, &other));
            assert_eq!(
                execution_phase(&other, profile, source, answered).err(),
                Some(ClosedSourceRefusal::Currency)
            );
        });
    }

    struct Silent;
    impl pack_hooks::PackHookHost for Silent {
        fn invoke(
            &self,
            _: pack_hooks::HookSlot,
            _: &pack_hooks::HookCall<'_>,
        ) -> pack_hooks::HookAnswer {
            pack_hooks::HookAnswer::Abstain
        }
    }

    #[test]
    fn a_host_installed_before_the_execution_phase_is_refused() {
        let packs = length_pack();
        on_worker(move || {
            let source = "puts [owner::f abc]\n";
            let (registry, answered) = answered(&packs, source);
            pack_hooks::install_host(std::rc::Rc::new(Silent));
            assert_eq!(
                execution_phase(&registry, owned_profile().unwrap(), source, answered).err(),
                Some(ClosedSourceRefusal::NativeContext)
            );
            pack_hooks::clear_host();
        });
    }
}
