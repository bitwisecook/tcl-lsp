// SPDX-License-Identifier: AGPL-3.0-or-later
//! Authentic pinned Jim original-object lookup policy.

impl crate::InvocationDialect {
    /// Actual pinned Jim lookup-cache policy; compatibility versions and C
    /// command-name caches cannot supply this original-object protocol.
    #[must_use]
    pub fn native_jim_lookup_protocol(
        self,
    ) -> Option<tcl_syntax::native_jim_lookup::NativeJimLookupProtocol> {
        (self.native_name_protocol()? == tcl_syntax::naming::NativeNameProtocol::Jim084)
            .then_some(tcl_syntax::native_jim_lookup::NativeJimLookupProtocol::jim084())
    }
}

/// Original object operations for Jim's global namespace-origin helper.
/// Canonicalisation retains the caller namespace; all getters use the actual
/// root namespace. Implementations retain their own provider and currency gates.
pub trait NativeJimNamespaceOriginBackend {
    /// Retained original header, independently of its reporting bytes.
    type Original: Clone;
    /// The adapter's original failure and host-refusal transport.
    type Error;
    /// Construct or retain the canonical operand in the actual caller namespace.
    fn canonical(&mut self, original: &Self::Original) -> Result<Self::Original, Self::Error>;
    /// Query the root alias flag and return the same original prefix object.
    fn alias_prefix(
        &mut self,
        canonical: &Self::Original,
    ) -> Result<Option<Self::Original>, Self::Error>;
    /// Query the original operand through the root command owner.
    fn command_exists(&mut self, original: &Self::Original) -> Result<bool, Self::Error>;
    /// Report the winning operand with Jim's counted root prefix.
    fn report(&mut self, original: &Self::Original) -> Result<Vec<u8>, Self::Error>;
    /// Compare actual header identity without comparing name strings.
    fn same_original(&self, left: &Self::Original, right: &Self::Original) -> bool;
    /// Refuse an unbounded alias cycle without supplying a guest result.
    fn alias_cycle_unavailable(&mut self) -> Self::Error;
}

/// A missing origin retains the actual recursive operand for its guest error.
pub enum NativeJimNamespaceOriginOutcome<V> {
    /// Counted report of the winning original operand.
    Found(Vec<u8>),
    /// Actual recursive operand used to report a missing command.
    Missing(V),
}

/// Execute the pinned helper's alias/canonical/original query order through
/// existing original-object getters. An alias prefix remains ONE name object.
///
/// # Errors
/// Preserves adapter refusals; a repeated actual prefix header refuses rather
/// than inventing a guest result for an unbounded original alias cycle.
pub fn native_jim_namespace_origin<B: NativeJimNamespaceOriginBackend>(
    backend: &mut B,
    original: &B::Original,
) -> Result<NativeJimNamespaceOriginOutcome<B::Original>, B::Error> {
    let mut original = original.clone();
    let mut prefixes = Vec::new();
    loop {
        let canonical = backend.canonical(&original)?;
        if let Some(prefix) = backend.alias_prefix(&canonical)? {
            if prefixes
                .iter()
                .any(|seen| backend.same_original(seen, &prefix))
            {
                return Err(backend.alias_cycle_unavailable());
            }
            prefixes.push(prefix.clone());
            original = prefix;
            continue;
        }
        if backend.command_exists(&canonical)? {
            return backend
                .report(&canonical)
                .map(NativeJimNamespaceOriginOutcome::Found);
        }
        if backend.command_exists(&original)? {
            return backend
                .report(&original)
                .map(NativeJimNamespaceOriginOutcome::Found);
        }
        return Ok(NativeJimNamespaceOriginOutcome::Missing(original));
    }
}
