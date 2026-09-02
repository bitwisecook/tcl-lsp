// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `SslicTcl` is the data and analysis owner for offline TLS assurance.
//!
//! The crate has no dependency on BIG-IP, a report renderer, an editor, or a
//! network stack.  Product-specific adapters project configuration into the
//! types in [`model`], then use the certificate, trust, chain, and estimate
//! engines here.  The `.sslictcl` language is parsed as Tcl syntax but is
//! never evaluated.
//!
//! The document pipeline is four separate phases, and each one can be used on
//! its own:
//!
//! 1. **Load** — [`dsl::load_with_diagnostics`] walks the concrete syntax tree
//!    and returns a [`model::SslicModel`] plus every [`dsl::DslDiagnostic`] it
//!    found, each with a published code and a byte range into the original
//!    document. It recovers: a bad declaration is skipped, not fatal.
//!    [`dsl::load`] is the first-error wrapper batch consumers use.
//! 2. **Estimate** — [`estimate::estimate`] scores an endpoint offline. A
//!    document's declared `protocol` / `cipher` facts reach it through
//!    [`estimate::EstimateInput::facts`] and override the built-in heuristics.
//! 3. **Evaluate** — [`policy::evaluate_policy`] applies a declared policy to
//!    one endpoint. It is never part of loading, and a check's `predicate`
//!    script is retained and never evaluated.
//! 4. **Emit** — [`model::SslicModel::to_sslictcl`] renders a model back to a
//!    deterministic document that loads to an equal model.
//!
//! [`vocabulary::DECLARATIONS`] states the whole vocabulary as data, and
//! `docs/design/sslictcl-vocabulary.md` is its prose reference.

pub mod certificate;
pub mod chain;
pub mod config_adapter;
pub mod dataset;
pub mod dsl;
mod emit;
pub mod estimate;
pub mod key;
pub mod model;
pub mod nginx;
pub mod openssl_config;
pub mod policy;
pub mod testssl;
pub mod trust;
pub mod vocabulary;

pub use certificate::{Certificate, CertificateError, parse_certificates};
pub use chain::{ChainEvaluation, ChainFinding, ChainFindingKind, ChainStatus, evaluate_chain};
pub use config_adapter::{ConfigEvidence, ConfigNotice};
pub use dataset::{
    DatasetError, ProgramAnchor, TrustProgramSnapshot, canonical_dataset_json,
    compile_trust_snapshots,
};
pub use dsl::{
    DslDiagnostic, DslDocument, DslError, DslLoad, DslNotice, DslSeverity, load,
    load_with_diagnostics,
};
pub use estimate::{
    Estimate, EstimateFinding, EstimateInput, EstimateSeverity, Grade, cipher_has_forward_secrecy,
    estimate,
};
pub use key::{KeyMatch, KeyMatchStatus, evaluate_private_key_match, private_key_spki_sha256};
pub use model::{
    CertificateDeclaration, ChainDeclaration, CipherFact, Endpoint, GradeRule, HstsPolicy, Policy,
    PolicyCheck, ProtocolFact, ProtocolVersion, SslicModel, TlsFacts, TlsStatus, TlsValue,
    TrustAnchorDeclaration, TrustProgramDeclaration,
};
pub use policy::{PolicyFinding, evaluate_policy};
pub use testssl::{TestSslFinding, TestSslImport, import_testssl_json};
pub use trust::{
    Anchor, ClientFamily, EmbeddedDataset, Provenance, TrustDecision, TrustPurpose, TrustStore,
    embedded_dataset,
};
pub use vocabulary::{DECLARATIONS, Declaration, Member, ValueDomain};
