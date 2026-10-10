// SPDX-License-Identifier: AGPL-3.0-or-later
//! Selected namespace-upvar argument grammar, independent of namespace lookup.

use crate::InvocationDialect;
use tcl_dialect::TclVersion;

/// The actual worker or Jim script helper that consumes the original operands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeNamespaceUpvarProtocol {
    /// C8.5 requires at least one complete target/local pair.
    Tcl85,
    /// C8.6 and later permit zero complete pairs.
    Tcl86Plus,
    /// Jim canonicalizes the namespace and forwards pairs to the `upvar` command.
    Jim084,
}

/// Original word positions accepted by the selected namespace-upvar grammar.
/// This projection grants neither namespace existence nor variable authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeNamespaceUpvarArguments {
    operand_count: usize,
}

impl NativeNamespaceUpvarArguments {
    /// Target and local positions after the namespace operand at position zero.
    /// Jim's final unpaired target has no local operand; its helper supplies empty.
    pub fn pairs(self) -> impl Iterator<Item = (usize, Option<usize>)> {
        (1..self.operand_count).step_by(2).map(move |target| {
            (
                target,
                (target + 1 < self.operand_count).then_some(target + 1),
            )
        })
    }
}

impl NativeNamespaceUpvarProtocol {
    /// Validate only the original operand count, including the namespace word.
    /// Jim accepts zero pairs here so its actual forwarded `upvar` reports arity.
    ///
    /// # Errors
    /// Returns the selected outer worker's usage when its grammar rejects.
    pub fn arguments(
        self,
        operand_count: usize,
    ) -> Result<NativeNamespaceUpvarArguments, &'static [u8]> {
        let accepted = match self {
            Self::Tcl85 => operand_count >= 3 && !operand_count.is_multiple_of(2),
            Self::Tcl86Plus => operand_count >= 1 && !operand_count.is_multiple_of(2),
            Self::Jim084 => operand_count >= 1,
        };
        if accepted {
            Ok(NativeNamespaceUpvarArguments { operand_count })
        } else {
            Err(self.wrong_arguments_usage())
        }
    }

    /// Usage belonging to the outer worker or Jim helper's formal arguments.
    #[must_use]
    pub const fn wrong_arguments_usage(self) -> &'static [u8] {
        match self {
            Self::Tcl85 => b"namespace upvar ns otherVar myVar ?otherVar myVar ...?",
            Self::Tcl86Plus => b"namespace upvar ns ?otherVar myVar ...?",
            Self::Jim084 => b"namespace upvar ns ?arg ...?",
        }
    }

    /// Release-specific suffix after the original namespace command prefix.
    /// Native wrong-argument publishers retain that original prefix separately.
    #[must_use]
    pub const fn wrong_arguments_suffix(self) -> &'static [u8] {
        match self {
            Self::Tcl85 => b"ns otherVar myVar ?otherVar myVar ...?",
            Self::Tcl86Plus => b"ns ?otherVar myVar ...?",
            Self::Jim084 => b"ns ?arg ...?",
        }
    }

    /// Jim's actual forwarded command usage, independent of the helper's formals.
    #[must_use]
    pub const fn forwarded_wrong_arguments_usage(self) -> Option<&'static [u8]> {
        match self {
            Self::Jim084 => Some(b"upvar ?level? otherVar myVar ?otherVar myVar ...?"),
            Self::Tcl85 | Self::Tcl86Plus => None,
        }
    }
}

impl InvocationDialect {
    /// Select an authenticated native worker grammar. C8.4 has no such member.
    #[must_use]
    pub fn native_namespace_upvar_protocol(self) -> Option<NativeNamespaceUpvarProtocol> {
        let protocol = self.native_name_protocol()?;
        if protocol.is_jim084() {
            return Some(NativeNamespaceUpvarProtocol::Jim084);
        }
        match protocol.tcl_version()? {
            TclVersion::V8_4 => None,
            TclVersion::V8_5 => Some(NativeNamespaceUpvarProtocol::Tcl85),
            TclVersion::V8_6 | TclVersion::V9_0 | TclVersion::V9_1 => {
                Some(NativeNamespaceUpvarProtocol::Tcl86Plus)
            }
        }
    }

    /// Authentic C namespace-upvar usage; Jim forwards to a separate command.
    #[must_use]
    pub fn namespace_upvar_wrong_arguments_usage(self) -> Option<&'static [u8]> {
        let protocol = self.native_namespace_upvar_protocol()?;
        match protocol {
            NativeNamespaceUpvarProtocol::Jim084 => None,
            NativeNamespaceUpvarProtocol::Tcl85 | NativeNamespaceUpvarProtocol::Tcl86Plus => {
                Some(protocol.wrong_arguments_usage())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_namespace_upvar_grammar_preserves_worker_and_forwarded_frontiers() {
        // Native proof naming.namespace.dispatch-upvar-zero-argument-usage:
        // docs/design/analysis/name-resolution-proofs/namespace-dispatch-upvar-zero-argument-usage.md
        assert!(
            InvocationDialect::for_version(TclVersion::V8_4)
                .native_namespace_upvar_protocol()
                .is_none()
        );
        for version in [
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let protocol = InvocationDialect::for_version(version)
                .native_namespace_upvar_protocol()
                .unwrap();
            assert!(protocol.arguments(0).is_err());
            assert_eq!(protocol.arguments(1).is_ok(), version != TclVersion::V8_5);
            assert!(protocol.arguments(2).is_err());
            assert_eq!(
                protocol.arguments(3).unwrap().pairs().collect::<Vec<_>>(),
                [(1, Some(2))]
            );
        }
        let jim = NativeNamespaceUpvarProtocol::Jim084;
        assert!(jim.arguments(0).is_err());
        assert_eq!(jim.arguments(1).unwrap().pairs().count(), 0);
        assert_eq!(
            jim.arguments(2).unwrap().pairs().collect::<Vec<_>>(),
            [(1, None)]
        );
        assert_eq!(
            jim.arguments(4).unwrap().pairs().collect::<Vec<_>>(),
            [(1, Some(2)), (3, None)]
        );
    }
}
