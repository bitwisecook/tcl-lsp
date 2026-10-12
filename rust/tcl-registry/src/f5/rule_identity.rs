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

//! Logical iRule ownership for the measured partition-root `call` contract.
//!
//! Rule identity comes from a configuration object or an explicit session
//! input. Source text and procedure spelling never manufacture an owner.

/// A canonical absolute BIG-IP folder path identifying an iRule.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RuleIdentity(String);

/// Invalid rule ownership or procedure target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleIdentityError {
    /// A rule requires an absolute path with a folder and a name.
    InvalidPath,
    /// A local or relative call requires an explicit owning rule.
    MissingOwner,
    /// The final procedure name is empty or contains a namespace separator.
    InvalidProcedure,
}

impl std::fmt::Display for RuleIdentityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidPath => "an iRule identity requires an absolute folder/name path",
            Self::MissingOwner => "an iRule call requires explicit rule ownership",
            Self::InvalidProcedure => "an iRule call requires an unqualified procedure name",
        })
    }
}

impl std::error::Error for RuleIdentityError {}

impl RuleIdentity {
    /// Validate an identity supplied by configuration or a caller.
    ///
    /// # Errors
    /// Returns [`RuleIdentityError::InvalidPath`] for a noncanonical path.
    pub fn new(path: impl Into<String>) -> Result<Self, RuleIdentityError> {
        let path = path.into();
        let components: Vec<_> = path.split('/').collect();
        if !path.starts_with('/')
            || components.len() < 3
            || path.contains("::")
            || components[1..]
                .iter()
                .any(|part| part.is_empty() || *part == "." || *part == "..")
        {
            return Err(RuleIdentityError::InvalidPath);
        }
        Ok(Self(path))
    }

    /// Canonical configuration object path.
    #[must_use]
    pub fn as_path(&self) -> &str {
        &self.0
    }

    /// Owning partition root, independent of the rule folder.
    #[must_use]
    pub fn partition(&self) -> &str {
        let end = self.0[1..].find('/').expect("validated rule identity") + 1;
        &self.0[..end]
    }

    /// Owning folder, including its partition.
    #[must_use]
    pub fn folder(&self) -> &str {
        self.0.rsplit_once('/').map_or("", |(folder, _)| folder)
    }
}

/// A documented logical rule/procedure target, independent of Tcl namespaces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleProcedureTarget {
    /// Explicit owner selected by the call.
    pub rule: RuleIdentity,
    /// Unqualified procedure name within that rule.
    pub procedure: String,
}

impl RuleProcedureTarget {
    /// Written rule component of a literal rule/procedure target, for source
    /// reports whose caller independently resolves configuration objects.
    /// Relative spellings remain relative; no partition or rule owner is
    /// fabricated. Local, malformed and unknown targets supply no candidate.
    #[must_use]
    pub fn referenced_rule_spelling(target: &str) -> Option<&str> {
        // Implementation contract: naming.consumer.original-rule-reference-candidates
        // docs/design/analysis/name-resolution-proofs/original-rule-reference-candidates.md
        let (rule, procedure) = target.rsplit_once("::")?;
        if rule.is_empty() || procedure.is_empty() || procedure.contains('/') {
            return None;
        }
        if rule.starts_with('/') {
            RuleIdentity::new(rule).ok()?;
        } else if rule.contains('/') || rule.contains("::") {
            return None;
        }
        Some(rule)
    }

    /// Resolve documented local, partition-root and absolute-folder call forms.
    ///
    /// # Errors
    /// Returns an error when identity is absent or the target is malformed.
    pub fn resolve(target: &str, owner: Option<&RuleIdentity>) -> Result<Self, RuleIdentityError> {
        let (rule, procedure) = if let Some((rule, procedure)) = target.rsplit_once("::") {
            let path = if rule.starts_with('/') {
                rule.to_owned()
            } else {
                let owner = owner.ok_or(RuleIdentityError::MissingOwner)?;
                if rule.contains('/') || rule.contains("::") {
                    return Err(RuleIdentityError::InvalidPath);
                }
                format!("{}/{rule}", owner.partition())
            };
            (RuleIdentity::new(path)?, procedure)
        } else {
            (
                owner.ok_or(RuleIdentityError::MissingOwner)?.clone(),
                target,
            )
        };
        if procedure.is_empty() || procedure.contains("::") || procedure.contains('/') {
            return Err(RuleIdentityError::InvalidProcedure);
        }
        Ok(Self {
            rule,
            procedure: procedure.to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn call_targets_preserve_rule_ownership_and_partition_root_lookup() {
        let current = RuleIdentity::new("/Common/folder/request").unwrap();
        for (target, rule) in [
            ("local", "/Common/folder/request"),
            ("helpers::local", "/Common/helpers"),
            ("/Common/folder/helpers::local", "/Common/folder/helpers"),
            ("/Other/helpers::local", "/Other/helpers"),
        ] {
            let resolved = RuleProcedureTarget::resolve(target, Some(&current)).unwrap();
            assert_eq!(resolved.rule.as_path(), rule);
            assert_eq!(resolved.procedure, "local");
        }
        assert_eq!(
            RuleProcedureTarget::resolve("local", None),
            Err(RuleIdentityError::MissingOwner)
        );
        assert!(RuleIdentity::new("/Common/../helpers").is_err());
        assert!(RuleIdentity::new("helpers").is_err());
        assert!(RuleProcedureTarget::resolve("::helpers::local", Some(&current)).is_err());
    }
    #[test]
    fn original_rule_spelling_candidates_do_not_fabricate_ownership() {
        // Implementation contract: naming.consumer.original-rule-reference-candidates
        // docs/design/analysis/name-resolution-proofs/original-rule-reference-candidates.md
        for (target, spelling) in [
            ("Lib::one", "Lib"),
            ("/Common/folder/Lib::one", "/Common/folder/Lib"),
        ] {
            assert_eq!(
                RuleProcedureTarget::referenced_rule_spelling(target),
                Some(spelling)
            );
        }
        for target in [
            "local",
            "::plain",
            "Lib::",
            "Lib::nested::one",
            "/Common/../Lib::one",
            "Lib::bad/name",
        ] {
            assert_eq!(RuleProcedureTarget::referenced_rule_spelling(target), None);
        }
        assert_eq!(
            RuleProcedureTarget::resolve("Lib::one", None),
            Err(RuleIdentityError::MissingOwner)
        );
    }
}
