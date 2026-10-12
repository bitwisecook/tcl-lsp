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

//! The version floor a **request-time** provider answers at.
//!
//! Every lifecycle-bearing fact in the registry — a command, a subcommand, an
//! option, an enumerable value, and a per-argument row — is
//! read through an accessor taking `package_version: Option<&str>`. The floor
//! itself is a per-document fact, so `tcl-registry` never holds one: its
//! handles are cached per (profile, pack overlay) and shared across documents.
//!
//! This is the provider-side half of that split, in one place so completion,
//! and any provider that follows it, cannot drift on what a document's floor
//! is. It is deliberately **request-time only**: the answer needs the whole
//! document's `package require` lines, which exist only once the walk that
//! records them has finished. A consumer running *during* the walk cannot use
//! this — that is why the arity gate defers its verdict to a post-walk flush
//! rather than asking mid-walk.

use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::signature_scan::types::SignaturePackageRequire;

/// One document's resolved-floor context, as a request-time provider sees it.
///
/// Copy-cheap (two references), so a provider threading it through helpers
/// pays nothing for passing it by value.
#[derive(Clone, Copy)]
pub struct DocumentFloor<'a> {
    analysis: &'a AnalysisResult,
    profile: &'static tcl_dialect::DialectProfile,
}

impl<'a> DocumentFloor<'a> {
    /// Bind the floor context to one analysed document under one profile.
    #[must_use]
    pub fn new(
        analysis: &'a AnalysisResult,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Self {
        Self { analysis, profile }
    }

    /// Retained availability, with compatibility only when lexical advice was
    /// independently selected. A caller profile cannot replace actual inputs.
    pub(crate) fn context(&self) -> Option<&'a tcl_registry::model::ResolvedContext> {
        self.analysis
            .resolved_input
            .as_ref()
            .map(tcl_compiler::analyser::ResolvedAnalysisInput::availability_context)
            .or_else(|| {
                self.analysis
                    .allows_lexical_declaration_advice()
                    .then(|| crate::document_context_for_profile(self.profile))
            })
    }

    /// Whether source authoring can suggest a package's catalogue. A written
    /// require is conditional advice; it proves no loader ran or package exists.
    pub(crate) fn advises_package(&self, package: &str) -> bool {
        self.context().is_some_and(|context| {
            context.can_host_package(package)
                && (context.ambient_package(package)
                    || self.analysis.package_requires.iter().any(|req| {
                        !req.conditional
                            && !req.control_flow
                            && self.requirement_names_package(req, package)
                    }))
        })
    }

    fn input_is_current(
        &self,
        input: &tcl_compiler::signature_scan::scope::SignatureSourceNameInput,
    ) -> bool {
        let Some(config) = self.analysis.body_lexer_config else {
            return false;
        };
        let Some(image) = self
            .analysis
            .retained_command_realm()
            .and_then(tcl_compiler::realm::CommandBindingRealm::original_source_image)
        else {
            return false;
        };
        let Some(container) = input.original_static_list_container() else {
            return false;
        };
        container.parent_word().image() == image
            && container.parent_word().config() == config
            && self.analysis.matches_original_source_image(image, config)
    }

    fn requirement_names_package(&self, req: &SignaturePackageRequire, package: &str) -> bool {
        if let Some(name) = &req.original_name {
            return name.key().matches_ascii(package) && self.input_is_current(name.input());
        }
        self.analysis.allows_lexical_declaration_advice() && req.name == package
    }

    fn requirement_floor(&self, req: &'a SignaturePackageRequire) -> Option<&'a str> {
        if req.original_name.is_some() {
            // Alternatives are OR: every admitted version is bounded by the
            // least requested floor. Independent require statements combine
            // by their greatest floor in for_spec.
            let mut floor = None;
            for input in &req.original_requirements {
                let input = input.as_ref()?;
                if !self.input_is_current(input) || input.bytes().contains(&0) {
                    return None;
                }
                let text = std::str::from_utf8(input.bytes()).ok()?;
                let lower = tcl_registry::version::requirement_lower_bound(text);
                floor = Some(floor.map_or(lower, |previous| {
                    if tcl_registry::version::compare(lower, previous).is_lt() {
                        lower
                    } else {
                        previous
                    }
                }));
            }
            return floor;
        }
        self.analysis
            .allows_lexical_declaration_advice()
            .then_some(())?;
        req.requirements
            .iter()
            .map(String::as_str)
            .chain(
                req.requirements
                    .is_empty()
                    .then_some(req.version.as_deref())
                    .flatten(),
            )
            .map(tcl_registry::version::requirement_lower_bound)
            .min_by(|a, b| tcl_registry::version::compare(a, b))
    }

    /// The source-advice version floor for `spec`'s owning package.
    ///
    /// The retained context's placement pin supplies the base floor (the shipped Tk
    /// on a plain Tcl base, a keyed vendor surface at its D5 oldest-supported
    /// default); an explicit `package require` can only **raise** it. When
    /// several requires name the same package, the most restrictive (highest)
    /// lower bound wins.
    ///
    /// Only *unconditional* source requests count: an optional probe
    /// (`catch {package require Tk 8.7}`, or a require inside an `if` arm)
    /// supplies no unconditional source requirement, so counting it would raise the floor
    /// and hide a gated option, value or argument that may not be there.
    ///
    /// These authoring bounds prove no package is loaded or installed.
    /// `None` when the command is not package-gated, or the package was
    /// required without a version — permissive, matching every other
    /// lifecycle query.
    #[must_use]
    pub fn for_spec(&self, spec: &tcl_registry::CommandSpec) -> Option<&'a str> {
        let context = self.context()?;
        let package = context
            .keyed_ambient_placement(spec)
            .map(|placement| placement.package.as_ref())
            .or_else(|| spec.owning_package())?;
        let require_floor = self
            .analysis
            .package_requires
            .iter()
            .filter(|req| {
                !req.conditional
                    && !req.control_flow
                    && self.requirement_names_package(req, package)
            })
            .filter_map(|req| self.requirement_floor(req))
            .max_by(|a, b| tcl_registry::version::compare(a, b));
        let pin_floor = context
            .placement_floor(package)
            .map(tcl_dialect::model::Version::as_str);
        match (pin_floor, require_floor) {
            (Some(pin), Some(required)) => {
                if tcl_registry::version::compare(required, pin).is_gt() {
                    Some(required)
                } else {
                    Some(pin)
                }
            }
            (pin, required) => pin.or(required),
        }
    }
}

#[cfg(test)]
mod original_package_advice_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn original_package_advice_keeps_actual_context_and_original_requirement_units() {
        // Implementation contract: naming.core.original-package-completion-advice
        // docs/design/analysis/name-resolution-proofs/original-package-completion-advice.md
        let source = "package require Tk 8.7\n";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        let profile = tcl_dialect::DialectProfile::find("f5-irules").unwrap();
        let registry = analysis.resolved_registry().unwrap();
        let spec = registry.get("entry").unwrap();
        assert!(analysis.package_requires[0].original_name.is_some());
        assert!(
            !analysis.package_requires[0]
                .original_requirements
                .is_empty()
        );
        assert_eq!(
            DocumentFloor::new(&analysis, profile).for_spec(spec),
            Some("8.7")
        );
        analysis.package_requires[0].name = "Other".to_owned();
        analysis.package_requires[0].version = Some("99".to_owned());
        analysis.package_requires[0].requirements = vec!["99".to_owned()];
        let registry = analysis.resolved_registry().unwrap();
        let spec = registry.get("entry").unwrap();
        let advice = DocumentFloor::new(&analysis, profile);
        assert!(advice.advises_package("Tk"));
        assert_eq!(advice.for_spec(spec), Some("8.7"));

        let mut plain = Analyser::new().analyse("butt\n", "tcl8.6");
        let mut unowned = analysis.package_requires[0].clone();
        unowned.original_name = None;
        unowned.original_requirements.clear();
        unowned.name = "Tk".to_owned();
        plain.package_requires.push(unowned);
        assert!(!DocumentFloor::new(&plain, profile).advises_package("Tk"));
        plain.package_requires = analysis.package_requires.clone();
        assert!(
            !DocumentFloor::new(&plain, profile).advises_package("Tk"),
            "foreign source rows cannot donate a package request"
        );
        let ambient = Analyser::new().analyse("butt\n", "tk");
        let wrong_profile = tcl_dialect::DialectProfile::find("tcl8.4").unwrap();
        assert!(DocumentFloor::new(&ambient, wrong_profile).advises_package("Tk"));
    }

    #[test]
    fn original_package_advice_preserves_alternative_floor_and_conditional_bounds() {
        // Implementation contract: naming.core.original-package-completion-advice
        // docs/design/analysis/name-resolution-proofs/original-package-completion-advice.md
        let source = "package require Tk 9.1 8.7\n";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        let profile = tcl_dialect::DialectProfile::find("tcl9.1").unwrap();
        let spec = analysis.resolved_registry().unwrap().get("entry").unwrap();
        assert_eq!(analysis.package_requires[0].original_requirements.len(), 2);
        assert_eq!(
            DocumentFloor::new(&analysis, profile).for_spec(spec),
            Some("8.7")
        );
        analysis.package_requires[0].conditional = true;
        let spec = analysis.resolved_registry().unwrap().get("entry").unwrap();
        let advice = DocumentFloor::new(&analysis, profile);
        assert!(!advice.advises_package("Tk"));
        assert_eq!(advice.for_spec(spec), Some("8.6"));
    }
}
