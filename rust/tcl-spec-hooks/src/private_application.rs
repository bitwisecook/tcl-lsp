// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! A hook host whose every engine is a private native core with memory-only
//! I/O, built after its owner's receiver is in place.
//!
//! This names the engine so the application owner does not have to. It is a
//! construction and retention seam only: it issues no closed-consumer scope,
//! and its owner must still check [`PrivateApplicationHost::is_quiet`] around
//! every selected evaluation.

use std::cell::RefCell;
use std::rc::Rc;

use tcl_engine_api::EngineError;
use tcl_engine_tclvm::{PrivateApplicationHookHost, TclVmEngine, private_application_hook_engine};
use tcl_registry::pack_hooks::{HookSlot, PackHookHost};

use crate::{CrashRecord, HookHost, HookInstallation, HostConfig, PackPrograms};

/// The private application VM, re-exported so its owner names no engine.
pub use tcl_engine_tclvm::PrivateApplicationVm;

/// The only release whose pinned hooks this host can run.
const PINNED_RELEASE: tcl_dialect::TclVersion = tcl_dialect::TclVersion::V9_1;

/// A [`HookHost`] over private engines, with every engine's memory host
/// retained until the owner drops this value.
pub struct PrivateApplicationHost {
    host: Rc<HookHost<TclVmEngine>>,
    roster: Rc<RefCell<Vec<TclVmEngine>>>,
    retained: Rc<RefCell<Vec<PrivateApplicationHookHost>>>,
}

impl PrivateApplicationHost {
    /// A host with no engine yet. Pinned engines are built on first use and
    /// only for [`PINNED_RELEASE`]; no caller engine, host or callback enters.
    #[must_use]
    pub fn new(config: HostConfig) -> Self {
        let roster: Rc<RefCell<Vec<TclVmEngine>>> = Rc::default();
        let retained: Rc<RefCell<Vec<PrivateApplicationHookHost>>> = Rc::default();
        let initial = Rc::clone(&roster);
        let pinned = Rc::clone(&retained);
        let host = HookHost::with_config(
            move || {
                initial
                    .borrow_mut()
                    .pop()
                    .expect("one private engine is prepared per installed pack")
            },
            config,
        )
        .with_pinned_engine_factory(move |profile| {
            let version = tcl_registry::model::ingress::resolve_known_environment(profile)
                .and_then(|environment| environment.catalogue_profile())
                .and_then(|profile| {
                    tcl_registry::InvocationDialect::of_profile(profile).tcl_version
                })
                .filter(|version| *version == PINNED_RELEASE)
                .ok_or(EngineError::Unsupported(
                    "private application pinned profile",
                ))?;
            let (engine, host) = private_application_hook_engine(version, true)
                .map_err(|_| EngineError::Unsupported("private application pinned bootstrap"))?;
            pinned.borrow_mut().push(host);
            Ok(engine)
        });
        Self {
            host: Rc::new(host),
            roster,
            retained,
        }
    }

    /// Build exactly `packs` initial engines, one for each later
    /// [`Self::install_pack_hooks`]. Returns `false`, building nothing more,
    /// when any private bootstrap is unavailable.
    #[must_use]
    pub fn prepare_initial_engines(&self, packs: usize) -> bool {
        for _ in 0..packs {
            let Ok((engine, host)) =
                private_application_hook_engine(tcl_dialect::TclVersion::V9_0, false)
            else {
                return false;
            };
            self.retained.borrow_mut().push(host);
            self.roster.borrow_mut().push(engine);
        }
        true
    }

    /// Whether every prepared initial engine has been taken by an install.
    #[must_use]
    pub fn initial_engines_spent(&self) -> bool {
        self.roster
            .try_borrow()
            .is_ok_and(|roster| roster.is_empty())
    }

    /// Compile one pack's hooks into the next prepared private engine.
    #[must_use]
    pub fn install_pack_hooks(&self, programs: PackPrograms) -> Vec<HookInstallation> {
        self.host.install_pack_hooks(programs)
    }

    /// The host as the registry's dispatch sees it.
    #[must_use]
    pub fn evaluator(&self) -> Rc<dyn PackHookHost> {
        Rc::clone(&self.host) as Rc<dyn PackHookHost>
    }

    /// Whether every retained memory host is still silent, has an empty
    /// environment and keeps its numeric state and integer ABI. Nothing is
    /// cleared or replaced to make this true.
    #[must_use]
    pub fn is_quiet(&self) -> bool {
        self.retained
            .try_borrow()
            .is_ok_and(|retained| retained.iter().all(|host| host.check_quiet_host().is_ok()))
    }

    /// Whether `slot`'s hook is quarantined or its pack poisoned.
    #[must_use]
    pub fn is_quarantined(&self, slot: HookSlot) -> bool {
        self.host.is_quarantined(slot)
    }

    /// Every hook error this host has logged.
    #[must_use]
    pub fn error_log(&self) -> Vec<String> {
        self.host.error_log()
    }

    /// Every crash this host has recorded.
    #[must_use]
    pub fn crash_records(&self) -> Vec<CrashRecord> {
        self.host.crash_records()
    }
}
