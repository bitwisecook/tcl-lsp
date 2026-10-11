// SPDX-License-Identifier: AGPL-3.0-or-later
//! Local script parser purpose, independent of the retained parent source owner.

use crate::LexerConfig;

/// The source producer's parser purpose within one original span space.
/// This selects lexical presentation only, never execution, a Native header,
/// a variable frame, a source entry or command implementation authority.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum SourceSegmentationPurpose {
    /// An independently supplied whole input retains its selected prologue rule.
    #[default]
    WholeInput,
    /// An extracted or materialised script treats its leading BOM as content.
    NestedScript,
}

impl SourceSegmentationPurpose {
    /// Select the local parser config while the caller retains the complete
    /// parent config for original words, metadata, source identity and caches.
    #[must_use]
    pub fn config(self, parent: LexerConfig) -> LexerConfig {
        match self {
            Self::WholeInput => parent,
            Self::NestedScript => parent.nested(),
        }
    }
}
