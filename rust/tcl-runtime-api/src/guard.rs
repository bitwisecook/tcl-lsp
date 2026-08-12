// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Runtime-issued guards for speculative compiler fast paths.
//!
//! Offline code generation can name the semantic implementation it expects,
//! but it cannot know a future interpreter's mutable epochs.  The live runtime
//! therefore verifies that expected identity, snapshots the requested mutation
//! domains, and issues an opaque [`GuardToken`].  Validation compares both the
//! live identity and every snapshot.  Epoch equality alone never authorises a
//! fast path.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

const DOMAIN_COUNT: usize = 7;

// Guard tokens cross runtime ABI boundaries as bare `u64` values. They must
// therefore identify one issuance process-wide, rather than merely within one
// interpreter: otherwise two live interpreters could both issue token 1 and
// accept each other's handle when their identity and epochs happen to match.
// Zero remains reserved for ABI-level decline.
static NEXT_GUARD_TOKEN: AtomicU64 = AtomicU64::new(1);

fn allocate_guard_token(counter: &AtomicU64) -> Option<GuardToken> {
    counter
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            current.checked_add(1)
        })
        .ok()
        .map(GuardToken)
}

/// Identity vocabulary allocated to the registry's intrinsic IDs. The identity
/// value is the registry intrinsic's explicit stable ID.
pub const REGISTRY_INTRINSIC_IDENTITY_NAMESPACE: u32 = 1;

/// Mutable interpreter domain whose stability can protect a fast path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum GuardDomain {
    /// Command definitions, renames, aliases, and imports.
    CommandEnvironment = 0,
    /// Namespace lookup paths and namespace membership.
    Namespace = 1,
    /// Namespace and interpreter unknown-command handling.
    UnknownHandling = 2,
    /// Variable trace registration.
    VariableTrace = 3,
    /// Command and execution trace registration.
    CommandTrace = 4,
    /// `TclOO` object and method dispatch state.
    ObjectDispatch = 5,
    /// Safe/child-interpreter policy and visibility.
    Interpreter = 6,
}

impl GuardDomain {
    const fn index(self) -> usize {
        self as usize
    }
}

/// A compact set of [`GuardDomain`] values.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct GuardDomains(u16);

impl GuardDomains {
    /// The empty set, which is not a legal guard request.
    pub const EMPTY: Self = Self(0);

    /// A set containing one domain.
    #[must_use]
    pub const fn one(domain: GuardDomain) -> Self {
        Self(1_u16 << domain as u8)
    }

    /// Decode the compact domain mask used by the code-generation ABI.
    ///
    /// Unknown high bits are rejected so a newer compiler cannot silently ask
    /// an older runtime to guard less state than intended.
    #[must_use]
    pub const fn from_bits(bits: u16) -> Option<Self> {
        let known = (1_u16 << DOMAIN_COUNT) - 1;
        if bits & !known == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Compact stable representation for ABI transport.
    #[must_use]
    pub const fn bits(self) -> u16 {
        self.0
    }

    /// Add a domain to this set.
    #[must_use]
    pub const fn with(mut self, domain: GuardDomain) -> Self {
        self.0 |= 1_u16 << domain as u8;
        self
    }

    /// Whether this set contains `domain`.
    #[must_use]
    pub const fn contains(self, domain: GuardDomain) -> bool {
        self.0 & (1_u16 << domain as u8) != 0
    }

    /// Whether no domains were selected.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

/// Stable semantic identity embedded by offline-generated code.
///
/// `namespace` allocates an identity vocabulary (for example, registry
/// intrinsic IDs); `value` identifies one implementation contract within it.
/// The runtime maps its live implementation to this stable identity.  It must
/// never derive identity solely from a command spelling or mutation epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GuardIdentity {
    namespace: u32,
    value: u64,
}

impl GuardIdentity {
    /// Construct a stable identity. Namespace zero is reserved as invalid.
    #[must_use]
    pub const fn new(namespace: u32, value: u64) -> Self {
        Self { namespace, value }
    }

    /// Identity of one registry-owned intrinsic stable ID.
    #[must_use]
    pub const fn registry_intrinsic(stable_id: u32) -> Self {
        Self::new(REGISTRY_INTRINSIC_IDENTITY_NAMESPACE, stable_id as u64)
    }

    /// Identity of one registry intrinsic under a release-semantics contract.
    ///
    /// The registry owns `semantics_key`; packing it beside the stable
    /// intrinsic ID prevents generated code for one runtime behaviour from
    /// entering a live implementation of another.
    #[must_use]
    pub const fn registry_intrinsic_with_semantics(stable_id: u32, semantics_key: u32) -> Self {
        if semantics_key == 0 {
            return Self::registry_intrinsic(stable_id);
        }
        Self::new(
            REGISTRY_INTRINSIC_IDENTITY_NAMESPACE,
            ((stable_id as u64) << 32) | semantics_key as u64,
        )
    }

    /// The identity-vocabulary namespace.
    #[must_use]
    pub const fn namespace(self) -> u32 {
        self.namespace
    }

    /// The identity within its vocabulary.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.value
    }

    const fn is_valid(self) -> bool {
        self.namespace != 0
    }
}

/// Opaque handle to one runtime-verified guard snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GuardToken(u64);

impl GuardToken {
    /// Reconstruct a token supplied over an ABI boundary.
    ///
    /// This does not make the token valid: [`GuardManager`] accepts only a
    /// currently-issued token from its own table.
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Stable scalar representation for an ABI transport.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// Why a runtime could not issue a guard token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardError {
    /// A guard must protect at least one mutable domain.
    EmptyDomains,
    /// The requested stable identity is invalid or has no live implementation.
    IdentityUnavailable,
    /// The live implementation does not have the identity expected by codegen.
    IdentityMismatch,
    /// Live mutable state does not satisfy a fast path's entry prerequisites.
    PrerequisiteUnsatisfied,
    /// At least one requested domain has been permanently poisoned.
    Poisoned,
    /// The token sequence was exhausted and the manager was poisoned.
    TokenExhausted,
}

impl fmt::Display for GuardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::EmptyDomains => "a guard must protect at least one domain",
            Self::IdentityUnavailable => "the live guard identity is unavailable",
            Self::IdentityMismatch => "the live implementation identity does not match",
            Self::PrerequisiteUnsatisfied => "live state does not satisfy guard prerequisites",
            Self::Poisoned => "a requested guard domain is poisoned",
            Self::TokenExhausted => "the guard token sequence is exhausted",
        })
    }
}

impl Error for GuardError {}

#[derive(Debug, Clone, Copy, Default)]
struct DomainState {
    epoch: u64,
    poisoned: bool,
}

#[derive(Debug, Clone)]
struct GuardRecord {
    identity: GuardIdentity,
    domains: GuardDomains,
    epochs: [u64; DOMAIN_COUNT],
}

/// Per-interpreter guard state and live-token table.
///
/// Mutation counters use checked arithmetic. Exhaustion poisons the affected
/// domain permanently, rather than wrapping to an old value and resurrecting a
/// stale token. Token exhaustion poisons the whole manager for the same reason.
#[derive(Debug)]
pub struct GuardManager {
    domains: [DomainState; DOMAIN_COUNT],
    tokens: BTreeMap<u64, GuardRecord>,
    poisoned: bool,
}

impl Default for GuardManager {
    fn default() -> Self {
        Self {
            domains: [DomainState::default(); DOMAIN_COUNT],
            tokens: BTreeMap::new(),
            poisoned: false,
        }
    }
}

impl GuardManager {
    /// Verify a live identity and issue a token over the selected domains.
    ///
    /// `observed` must come from live runtime resolution. `None` means the
    /// runtime cannot prove that the requested implementation is installed.
    pub fn prepare(
        &mut self,
        expected: GuardIdentity,
        observed: Option<GuardIdentity>,
        domains: GuardDomains,
    ) -> Result<GuardToken, GuardError> {
        if domains.is_empty() {
            return Err(GuardError::EmptyDomains);
        }
        if !expected.is_valid() || observed.is_none() {
            return Err(GuardError::IdentityUnavailable);
        }
        if observed != Some(expected) {
            return Err(GuardError::IdentityMismatch);
        }
        if self.poisoned
            || GuardDomain::ALL
                .iter()
                .any(|domain| domains.contains(*domain) && self.domains[domain.index()].poisoned)
        {
            return Err(GuardError::Poisoned);
        }
        let Some(token) = allocate_guard_token(&NEXT_GUARD_TOKEN) else {
            self.poison_all();
            return Err(GuardError::TokenExhausted);
        };
        let mut epochs = [0; DOMAIN_COUNT];
        for domain in GuardDomain::ALL {
            if domains.contains(domain) {
                epochs[domain.index()] = self.domains[domain.index()].epoch;
            }
        }
        self.tokens.insert(
            token.0,
            GuardRecord {
                identity: expected,
                domains,
                epochs,
            },
        );
        Ok(token)
    }

    /// Validate a token against the current live implementation identity.
    #[must_use]
    pub fn check(&self, token: GuardToken, observed: Option<GuardIdentity>) -> bool {
        if self.poisoned {
            return false;
        }
        let Some(record) = self.tokens.get(&token.0) else {
            return false;
        };
        if observed != Some(record.identity) {
            return false;
        }
        GuardDomain::ALL.iter().all(|domain| {
            !record.domains.contains(*domain)
                || (!self.domains[domain.index()].poisoned
                    && record.epochs[domain.index()] == self.domains[domain.index()].epoch)
        })
    }

    /// Validate a token for one exact expected identity.
    ///
    /// An ABI caller must not be able to reuse a valid token issued for one
    /// intrinsic form to admit another form that happens to resolve to the
    /// same command head.
    #[must_use]
    pub fn check_expected(
        &self,
        token: GuardToken,
        expected: GuardIdentity,
        observed: Option<GuardIdentity>,
    ) -> bool {
        self.tokens
            .get(&token.0)
            .is_some_and(|record| record.identity == expected)
            && self.check(token, observed)
    }

    /// Release a live token. Returns `false` for stale or already-released input.
    pub fn release(&mut self, token: GuardToken) -> bool {
        self.tokens.remove(&token.0).is_some()
    }

    /// Invalidate every token that snapshots `domain`.
    pub fn invalidate(&mut self, domain: GuardDomain) {
        let state = &mut self.domains[domain.index()];
        match state.epoch.checked_add(1) {
            Some(epoch) => state.epoch = epoch,
            None => state.poisoned = true,
        }
    }

    /// Permanently poison one domain when mutation coverage becomes uncertain.
    pub fn poison(&mut self, domain: GuardDomain) {
        self.domains[domain.index()].poisoned = true;
    }

    /// Permanently disable all tokens and token issuance.
    pub fn poison_all(&mut self) {
        self.poisoned = true;
        self.tokens.clear();
    }

    /// Number of currently live tokens, primarily for lifecycle diagnostics.
    #[must_use]
    pub fn active_token_count(&self) -> usize {
        self.tokens.len()
    }
}

impl GuardDomain {
    const ALL: [Self; DOMAIN_COUNT] = [
        Self::CommandEnvironment,
        Self::Namespace,
        Self::UnknownHandling,
        Self::VariableTrace,
        Self::CommandTrace,
        Self::ObjectDispatch,
        Self::Interpreter,
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXPECTED: GuardIdentity = GuardIdentity::new(1, 17);
    const OTHER: GuardIdentity = GuardIdentity::new(1, 18);
    const COMMAND: GuardDomains = GuardDomains::one(GuardDomain::CommandEnvironment);

    #[test]
    fn identity_is_checked_at_issue_and_validation() {
        let mut manager = GuardManager::default();
        assert_eq!(
            manager.prepare(EXPECTED, Some(OTHER), COMMAND),
            Err(GuardError::IdentityMismatch)
        );
        let token = manager.prepare(EXPECTED, Some(EXPECTED), COMMAND).unwrap();
        assert!(manager.check(token, Some(EXPECTED)));
        assert!(!manager.check(token, Some(OTHER)));
        assert!(!manager.check(token, None));
    }

    #[test]
    fn mutation_stales_only_tokens_covering_that_domain() {
        let mut manager = GuardManager::default();
        let command = manager.prepare(EXPECTED, Some(EXPECTED), COMMAND).unwrap();
        let traces = manager
            .prepare(
                EXPECTED,
                Some(EXPECTED),
                GuardDomains::one(GuardDomain::CommandTrace),
            )
            .unwrap();
        manager.invalidate(GuardDomain::CommandEnvironment);
        assert!(!manager.check(command, Some(EXPECTED)));
        assert!(manager.check(traces, Some(EXPECTED)));
    }

    #[test]
    fn release_is_exact_and_cannot_be_reused() {
        let mut manager = GuardManager::default();
        let token = manager.prepare(EXPECTED, Some(EXPECTED), COMMAND).unwrap();
        assert_eq!(manager.active_token_count(), 1);
        assert!(manager.release(token));
        assert!(!manager.release(token));
        assert!(!manager.check(token, Some(EXPECTED)));
        assert_eq!(manager.active_token_count(), 0);
    }

    #[test]
    fn token_from_another_manager_is_rejected() {
        let mut first = GuardManager::default();
        let mut second = GuardManager::default();
        let first_token = first.prepare(EXPECTED, Some(EXPECTED), COMMAND).unwrap();
        let second_token = second.prepare(EXPECTED, Some(EXPECTED), COMMAND).unwrap();

        assert_ne!(first_token, second_token);
        assert!(first.check(first_token, Some(EXPECTED)));
        assert!(second.check(second_token, Some(EXPECTED)));
        assert!(!first.check(second_token, Some(EXPECTED)));
        assert!(!second.check(first_token, Some(EXPECTED)));
        assert!(!first.release(second_token));
        assert!(!second.release(first_token));
    }

    #[test]
    fn domain_poisoning_fails_closed() {
        let mut manager = GuardManager::default();
        let token = manager.prepare(EXPECTED, Some(EXPECTED), COMMAND).unwrap();
        manager.poison(GuardDomain::CommandEnvironment);
        assert!(!manager.check(token, Some(EXPECTED)));
        assert_eq!(
            manager.prepare(EXPECTED, Some(EXPECTED), COMMAND),
            Err(GuardError::Poisoned)
        );
    }

    #[test]
    fn counter_exhaustion_cannot_resurrect_old_tokens() {
        let mut manager = GuardManager::default();
        let token = manager.prepare(EXPECTED, Some(EXPECTED), COMMAND).unwrap();
        manager.domains[GuardDomain::CommandEnvironment.index()].epoch = u64::MAX;
        manager.invalidate(GuardDomain::CommandEnvironment);
        assert!(!manager.check(token, Some(EXPECTED)));

        let exhausted = AtomicU64::new(u64::MAX);
        assert_eq!(allocate_guard_token(&exhausted), None);

        // The same failure path used by `prepare` poisons the affected manager;
        // keep the process-global allocator intact so this test is isolated.
        let mut manager = GuardManager::default();
        if allocate_guard_token(&exhausted).is_none() {
            manager.poison_all();
        }
        assert!(manager.poisoned);
        assert_eq!(manager.active_token_count(), 0);
    }
}
