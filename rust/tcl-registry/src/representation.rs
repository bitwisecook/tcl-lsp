// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Tcl value-representation effects declared by command specifications.
//!
//! Tcl values may carry both a string representation and a typed internal
//! representation. They are also reference-counted: a container mutation
//! duplicates a shared value before changing it. Registry consumers need
//! these facts for shimmer and copy-on-write diagnostics without recognising
//! command spellings.

/// A command form's effect on a Tcl value's representation or sharing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RepresentationEffect {
    /// No representation effect has been declared.
    #[default]
    None,
    /// Mutating the named container duplicates it when its Tcl object is
    /// shared.
    ///
    /// Both indices are relative to the effective command or subcommand's
    /// arguments. `minimum_arguments` distinguishes a mutation from a
    /// read-only or replacement spelling of the same command.
    CopyOnWriteContainerMutation {
        /// Argument containing the variable name whose value is mutated.
        variable_arg: u8,
        /// Minimum number of arguments required for this form to mutate the
        /// container in place.
        minimum_arguments: u8,
    },
}

impl RepresentationEffect {
    /// Declare a shared-container mutation.
    #[must_use]
    pub const fn copy_on_write_container(variable_arg: u8, minimum_arguments: u8) -> Self {
        Self::CopyOnWriteContainerMutation {
            variable_arg,
            minimum_arguments,
        }
    }

    /// Return the effective argument index carrying the mutated variable.
    ///
    /// `argument_offset` is one for a subcommand and zero for a top-level
    /// command. The invocation remains unclassified when it does not meet the
    /// descriptor's mutation arity floor.
    #[must_use]
    pub fn mutation_target_index(
        self,
        effective_argument_count: usize,
        argument_offset: usize,
    ) -> Option<usize> {
        match self {
            Self::CopyOnWriteContainerMutation {
                variable_arg,
                minimum_arguments,
            } if effective_argument_count >= usize::from(minimum_arguments) => {
                Some(argument_offset + usize::from(variable_arg))
            }
            Self::None | Self::CopyOnWriteContainerMutation { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_on_write_effect_applies_only_to_mutating_arity() {
        let effect = RepresentationEffect::copy_on_write_container(0, 2);
        assert_eq!(effect.mutation_target_index(1, 0), None);
        assert_eq!(effect.mutation_target_index(2, 0), Some(0));
        assert_eq!(effect.mutation_target_index(2, 1), Some(1));
    }
}
