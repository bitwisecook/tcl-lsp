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

//! The host a compiled C extension's commands are evaluated on: an extension
//! built for `wasm32` as a side module, loaded into the WASM runtime and run
//! under fuel (`docs/design/runtime/c-extension-abi.md` § 12).
//!
//! This module is the seam, and it holds no engine: the registry, and
//! everything that links it — the language server among them — never links a
//! WASM engine. A process that has one installs it for its thread
//! ([`install_extension_host`]), as the hook host is installed for pack hooks,
//! and a thread with none answers every load and evaluation with
//! [`DeclineReason::Transient`]: the host is unavailable, which is never a
//! negative and never cached as one.
//!
//! An extension is named by its artefact's content hash ([`artefact_hash`]), so
//! two artefacts that differ in a byte are never answered for each other, and
//! an evaluation is one command of one loaded extension, its words exact
//! values, under an [`ImplementationBudget`].

use std::cell::RefCell;
use std::rc::Rc;

use crate::value_transfer::{DeclineReason, ImplementationBudget};

/// An extension a host has loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedExtension {
    /// The artefact's content hash ([`artefact_hash`]), which names it.
    pub hash: u64,
    /// The prefix its entry point is named by (`Pkga` for `Pkga_Init`).
    pub prefix: String,
    /// The commands its entry point registered, in name order.
    pub commands: Vec<String>,
}

/// A host that loads compiled extensions and evaluates their commands.
///
/// Each evaluation is the extension's alone: it runs on an interpreter of its
/// own with the extension loaded into it, so no evaluation sees what another
/// left, and the host's clock, filesystem, randomness and environment are not
/// the extension's to read.
pub trait ExtensionHost {
    /// Load `artefact`, an extension built for this host, and run its entry
    /// point `PREFIX_Init`, answering what it registered.
    ///
    /// # Errors
    ///
    /// Why it cannot be loaded: [`DeclineReason::Unsupported`] for an artefact
    /// the host cannot link or an entry point that fails, and a
    /// [`DeclineReason::Budget`] for one that outruns the host's own budget.
    fn load(&self, artefact: &[u8], prefix: &str) -> Result<LoadedExtension, DeclineReason>;

    /// Evaluate one command of the loaded extension `hash`: `words` are the
    /// command's name and its arguments, run under `budget`. Answers the
    /// command's result.
    ///
    /// # Errors
    ///
    /// Why there is no result: a [`DeclineReason::Budget`] naming the limit
    /// the evaluation outran, [`DeclineReason::Unsupported`] for a command that
    /// raised an error (an error is never a value), and
    /// [`DeclineReason::Transient`] for an extension the host has not loaded.
    fn evaluate(
        &self,
        hash: u64,
        words: &[String],
        budget: &ImplementationBudget,
    ) -> Result<String, DeclineReason>;
}

thread_local! {
    /// The host serving this thread's extension evaluations. Thread-local
    /// because a host owns engine state that is not `Send`.
    static HOST: RefCell<Option<Rc<dyn ExtensionHost>>> = const { RefCell::new(None) };
}

/// Install `host` for this thread.
pub fn install_extension_host(host: Rc<dyn ExtensionHost>) {
    HOST.with(|slot| *slot.borrow_mut() = Some(host));
}

/// Remove this thread's host.
pub fn clear_extension_host() {
    HOST.with(|slot| *slot.borrow_mut() = None);
}

/// Whether this thread has a host.
#[must_use]
pub fn extension_host_installed() -> bool {
    HOST.with(|slot| slot.borrow().is_some())
}

/// This thread's host, held outside the slot's borrow so a host may consult
/// the slot while it runs.
fn host() -> Option<Rc<dyn ExtensionHost>> {
    HOST.with(|slot| slot.borrow().clone())
}

/// Load `artefact` on this thread's host ([`ExtensionHost::load`]).
///
/// # Errors
///
/// [`DeclineReason::Transient`] when the thread has no host, and the host's
/// own reason otherwise.
pub fn load_extension(artefact: &[u8], prefix: &str) -> Result<LoadedExtension, DeclineReason> {
    host().map_or(Err(DeclineReason::Transient), |host| {
        host.load(artefact, prefix)
    })
}

/// Evaluate a command of a loaded extension on this thread's host
/// ([`ExtensionHost::evaluate`]).
///
/// # Errors
///
/// [`DeclineReason::Transient`] when the thread has no host, and the host's
/// own reason otherwise.
pub fn evaluate_extension(
    hash: u64,
    words: &[String],
    budget: &ImplementationBudget,
) -> Result<String, DeclineReason> {
    host().map_or(Err(DeclineReason::Transient), |host| {
        host.evaluate(hash, words, budget)
    })
}

/// The content hash an extension artefact is named by: its bytes, through the
/// hasher every implementation's content goes through
/// ([`crate::implementation_hash::content_hash`]), so two artefacts that differ
/// in a byte are two extensions. It is the same rule as an implementation
/// identity's `content_hash`, not yet the same identity: the extension
/// evaluation route and the shipped implementation's attestation bind the two.
#[must_use]
pub fn artefact_hash(artefact: &[u8]) -> u64 {
    crate::implementation_hash::content_hash(artefact)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A host that loads anything as one command and echoes its words.
    struct Echo;

    impl ExtensionHost for Echo {
        fn load(&self, artefact: &[u8], prefix: &str) -> Result<LoadedExtension, DeclineReason> {
            Ok(LoadedExtension {
                hash: artefact_hash(artefact),
                prefix: prefix.to_owned(),
                commands: vec!["echo".to_owned()],
            })
        }

        fn evaluate(
            &self,
            _hash: u64,
            words: &[String],
            budget: &ImplementationBudget,
        ) -> Result<String, DeclineReason> {
            if budget.commands == Some(0) {
                return Err(DeclineReason::Budget(
                    crate::value_transfer::BudgetLimit::Fuel,
                ));
            }
            Ok(words.join(" "))
        }
    }

    /// With no host installed every load and evaluation is unavailable, and
    /// never a negative; with one installed each reaches it, and its answers
    /// and declines pass through unchanged.
    #[test]
    fn a_thread_without_a_host_declines_as_transient() {
        clear_extension_host();
        assert!(!extension_host_installed());
        let budget = ImplementationBudget::default();
        assert_eq!(
            load_extension(b"\0asm", "Pkga"),
            Err(DeclineReason::Transient)
        );
        assert_eq!(
            evaluate_extension(1, &["echo".to_owned()], &budget),
            Err(DeclineReason::Transient)
        );

        install_extension_host(Rc::new(Echo));
        assert!(extension_host_installed());
        let loaded = load_extension(b"\0asm", "Pkga").expect("the host loads it");
        assert_eq!(loaded.hash, artefact_hash(b"\0asm"));
        assert_eq!(
            evaluate_extension(loaded.hash, &["echo".to_owned(), "a".to_owned()], &budget),
            Ok("echo a".to_owned())
        );
        let spent = ImplementationBudget {
            commands: Some(0),
            ..ImplementationBudget::default()
        };
        assert_eq!(
            evaluate_extension(loaded.hash, &["echo".to_owned()], &spent),
            Err(DeclineReason::Budget(
                crate::value_transfer::BudgetLimit::Fuel
            ))
        );
        clear_extension_host();
        assert_eq!(
            load_extension(b"\0asm", "Pkga"),
            Err(DeclineReason::Transient)
        );
    }

    /// Two artefacts that differ in one byte are two extensions.
    #[test]
    fn an_artefact_is_named_by_its_bytes() {
        assert_eq!(artefact_hash(b"\0asm\x01"), artefact_hash(b"\0asm\x01"));
        assert_ne!(artefact_hash(b"\0asm\x01"), artefact_hash(b"\0asm\x02"));
    }
}
