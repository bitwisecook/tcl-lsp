// SPDX-License-Identifier: AGPL-3.0-or-later
//! Selected TclOO declaration-map role and effect stages.
//!
//! The embedding retains the genuine target, activation, original arguments and
//! currency. This coordinator supplies only the inspected C worker sequence;
//! it creates no target lookup, Native reference or execution authority.
use crate::CmdError;
use std::collections::BTreeSet;

/// Actual object-reference and map-storage adapters for a selected C worker.
///
/// Incoming descriptors initially carry allocation-only original observations.
/// `acquire_member` alone issues each source incoming reference. A retirement
/// keeps the old mapping visible through allocation-only observations while its
/// transferred stored roles are released outside the embedding's table borrow.
pub trait NativeOoVariableMappingOps {
    /// One incoming or installed declaration record, preserving original identity.
    type Member;
    /// The original old-visible mapping and matching mandatory cleanup ticket.
    type Retirement;
    /// Check genuine original target/activation/arguments/currency.
    fn check_mapping(&mut self) -> Result<(), CmdError>;
    /// Retain the full earliest typed execution refusal before owned cleanup.
    fn retain_mapping_failure(&mut self, error: &CmdError);
    /// Acquire the source's one actual incoming member reference.
    fn acquire_member(&mut self, original: &Self::Member) -> Result<Self::Member, CmdError>;
    /// Extract actual old roles under a short table access, leaving old visibility.
    fn capture_old_mapping(&mut self) -> Result<Self::Retirement, CmdError>;
    /// Retire every transferred old stored role outside table accesses.
    ///
    /// Mandatory release continues after the first cause; later optional work
    /// is stopped by the returned failure or subsequent original check.
    fn retire_old_mapping(&mut self, retired: &mut Self::Retirement) -> Result<(), CmdError>;
    /// End old visibility and establish the source's zero-length building map.
    fn begin_new_mapping(&mut self, retired: &mut Self::Retirement) -> Result<(), CmdError>;
    /// Run the canonical reached original String/hash-key stage.
    fn member_key(&mut self, original: &Self::Member) -> Result<Vec<u8>, CmdError>;
    /// Prepare a unique installed record, including a real private full-name role.
    ///
    /// A consuming adapter retains any full typed error before releasing the passed
    /// role on failure. Standard records simply transfer the incoming role. The selected private
    /// worker creates its counted full-name object only at this reached stage.
    fn prepare_unique_member(
        &mut self,
        original: Self::Member,
        key: &[u8],
    ) -> Result<Self::Member, CmdError>;
    /// Adopt actual unique records and publish their final count in the same map.
    /// A consuming adapter retains any full typed error before unused-role cleanup.
    fn publish_mapping(
        &mut self,
        members: Vec<Self::Member>,
        retired: &mut Self::Retirement,
    ) -> Result<(), CmdError>;
    /// Finish only matching mandatory retirement state, preserving replacements.
    fn finish_mapping_retirement(
        &mut self,
        retired: Option<Self::Retirement>,
    ) -> Result<(), CmdError>;
}

fn check_mapping<O: NativeOoVariableMappingOps>(ops: &mut O) -> Result<(), CmdError> {
    let reached = ops.check_mapping();
    if let Err(error) = &reached {
        ops.retain_mapping_failure(error);
    }
    reached
}

/// Install the selected original C declaration map through one stage worker.
///
/// Every incoming reference precedes old-visible retirement. Canonical original
/// hashing follows retirement and zero-length setup, with first-unique identity
/// retained. All reached Results are checked before further effects; complete
/// typed failures are retained before actual unused member/ticket cleanup.
///
/// # Errors
/// Returns an authentic adapter failure or the earliest retained execution
/// refusal. No compatibility or unavailable provider is silently selected.
pub fn install_original_variable_mapping<O: NativeOoVariableMappingOps>(
    ops: &mut O,
    originals: &[O::Member],
) -> Result<(), CmdError> {
    let mut incoming = Vec::new();
    let mut members = Vec::new();
    let mut retirement = None;
    let reached = (|| {
        check_mapping(ops)?;
        for original in originals {
            let acquired = ops.acquire_member(original);
            if let Err(error) = &acquired {
                ops.retain_mapping_failure(error);
            }
            check_mapping(ops)?;
            incoming.push(Some(acquired?));
        }
        let captured = ops.capture_old_mapping();
        if let Err(error) = &captured {
            ops.retain_mapping_failure(error);
        }
        retirement = Some(captured?);
        check_mapping(ops)?;
        let retired = retirement.as_mut().expect("actual captured mapping");
        let released = ops.retire_old_mapping(retired);
        if let Err(error) = &released {
            ops.retain_mapping_failure(error);
        }
        check_mapping(ops)?;
        released?;
        let cleared = ops.begin_new_mapping(retired);
        if let Err(error) = &cleared {
            ops.retain_mapping_failure(error);
        }
        check_mapping(ops)?;
        cleared?;
        let mut unique = BTreeSet::new();
        for original in &mut incoming {
            let actual = original.as_ref().expect("actual incoming role");
            let key = ops.member_key(actual);
            if let Err(error) = &key {
                ops.retain_mapping_failure(error);
            }
            check_mapping(ops)?;
            let key = key?;
            if unique.insert(key.clone()) {
                let prepared = ops.prepare_unique_member(
                    original.take().expect("actual unique incoming role"),
                    &key,
                );
                if let Err(error) = &prepared {
                    ops.retain_mapping_failure(error);
                }
                check_mapping(ops)?;
                members.push(prepared?);
            } else {
                drop(original.take());
                check_mapping(ops)?;
            }
        }
        let published = ops.publish_mapping(std::mem::take(&mut members), retired);
        if let Err(error) = &published {
            ops.retain_mapping_failure(error);
        }
        check_mapping(ops)?;
        published
    })();
    if let Err(error) = &reached {
        ops.retain_mapping_failure(error);
    }
    // No storage borrow survives while real unused roles leave the coordinator.
    drop(members);
    drop(incoming);
    let cleanup = ops.finish_mapping_retirement(retirement);
    if let Err(error) = &cleanup {
        ops.retain_mapping_failure(error);
    }
    check_mapping(ops)?;
    reached?;
    cleanup
}

#[cfg(test)]
mod tests;
