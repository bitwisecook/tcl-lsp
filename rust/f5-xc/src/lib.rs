//! iRule → F5 Distributed Cloud (XC) static translator.
//!
//! Translates F5 BIG-IP iRules into XC-native constructs (L7 routes,
//! service policies, header processing, origin pool references) by walking
//! the IR produced by [`tcl_compiler::lowering::lower_to_ir`], and emits the
//! **XC100-301** translatability diagnostics for the LSP pipeline.
//!
//! Public API
//! ----------
//! - [`translate_irule`] — analyse an iRule and return an
//!   [`XCTranslationResult`].
//! - [`get_xc_diagnostics`] — analyse an iRule and return XC-series
//!   [`XcDiagnostic`]s.

#![deny(missing_docs)]
#![forbid(unsafe_code)]
// Several pedantic lints fight this crate's structure: nested conditionals
// that follow the iRule `if`/`match` nesting, declarative data-table `match`
// arms that legitimately share a description string, and the long IR-walk
// dispatch over statement kinds.
#![allow(clippy::collapsible_if)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::too_many_lines)]

pub mod diagnostics;
pub mod mapping;
pub mod model;
pub mod translator;

pub use diagnostics::{XcDiagnostic, XcSeverity, get_xc_diagnostics};
pub use model::{TranslateStatus, XCConstructKind, XCTranslationResult};
pub use translator::{translate_irule, translate_irule_with_registry};
