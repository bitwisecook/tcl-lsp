//! Explicit effects of an original object's resident-string allocation owner.

/// Actual resident-string mutation reached before primary-cache publication.
/// Equal bytes do not establish allocation preservation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResidentStringMutation {
    /// No string allocation mutation occurred; retain the original allocation.
    Preserve,
    /// The allocation owner produced or replaced the resident string.
    Replace,
    /// The allocation owner discarded the resident string.
    Discard,
}
