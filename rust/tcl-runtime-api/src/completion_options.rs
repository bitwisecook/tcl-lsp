// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Standard Tcl return-option planning shared by runtime engines.
//!
//! The plan decides which standard keys a completion exposes and how live
//! error metadata overrides a carried `return -options` dictionary. Concrete
//! runtimes remain responsible only for constructing their Tcl object type.

use tcl_dialect::TclVersion;

use crate::{ArrayReadMiss, Code};

/// Whether a control-command activation inherits the surrounding completion's
/// carried options or begins with a fresh option set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivationOptionScope {
    /// The body is compiled transparently into the surrounding completion.
    Inherited,
    /// The body is a distinct Tcl command activation with fresh options.
    Fresh,
}

impl ActivationOptionScope {
    /// Whether this activation starts without surrounding carried options.
    #[must_use]
    pub const fn begins_fresh(self) -> bool {
        matches!(self, Self::Fresh)
    }
}

/// How a successful control command settles the options produced by its body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuccessOptionSettlement {
    /// Forward the options produced by the selected or final body activation.
    Forward,
    /// Produce an ordinary successful completion with no carried options.
    Settle,
}

/// Completion-option policy for a Tcl control-command activation.
///
/// Result flow and option flow are deliberately separate. For example,
/// `lmap` manufactures a list result but forwards the final iteration's
/// options, while `foreach` manufactures both its result and successful
/// options. Keeping the two option axes here gives bytecode and tree-walking
/// runtimes one vocabulary for those boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlOptionPolicy {
    /// The option scope in which a nested body starts.
    pub activation: ActivationOptionScope,
    /// The successful command's final option settlement.
    pub success: SuccessOptionSettlement,
}

impl ControlOptionPolicy {
    /// A body compiled transparently in the surrounding completion.
    pub const TRANSPARENT: Self = Self {
        activation: ActivationOptionScope::Inherited,
        success: SuccessOptionSettlement::Forward,
    };

    /// A fresh body activation whose produced options become the command's.
    pub const FRESH_FORWARDED: Self = Self {
        activation: ActivationOptionScope::Fresh,
        success: SuccessOptionSettlement::Forward,
    };

    /// A fresh activation whose successful completion is settled by its owner.
    pub const FRESH_SETTLED: Self = Self {
        activation: ActivationOptionScope::Fresh,
        success: SuccessOptionSettlement::Settle,
    };

    /// Whether the nested body must start without surrounding carried options.
    #[must_use]
    pub const fn begins_fresh(self) -> bool {
        self.activation.begins_fresh()
    }

    /// Whether an ordinary successful owner completion discards body options.
    #[must_use]
    pub const fn settles_success(self) -> bool {
        matches!(self.success, SuccessOptionSettlement::Settle)
    }
}

/// A planned option value which an engine materialises in its Tcl value model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptionValue<V> {
    /// A standard integer value (`-code`, `-level`, or `-errorline`).
    Integer(i64),
    /// An already-materialised runtime value.
    Value(V),
}

/// Live metadata for an error completion.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ErrorOptions<V> {
    /// The resolved Tcl error-code list. Every error supplies one.
    pub error_code: Option<V>,
    /// The accumulated `errorInfo`, once the error is active at level zero.
    pub error_info: Option<V>,
    /// The accumulated TIP 348 stack, once active and available in the release.
    pub error_stack: Option<V>,
    /// The source line of the innermost error frame.
    pub error_line: Option<i64>,
    /// The prior completion superseded by a `try` handler or `finally` error.
    pub during: Option<V>,
}

/// One carried return-option pair.
pub type CarriedOption<V> = (Vec<u8>, V);

/// Materialise the standard option pairs left by a failed `array get`
/// candidate read on the surrounding successful completion.
///
/// This is deliberately a completion-options operation rather than adapter
/// policy: native and standalone runtimes supply only their value constructor,
/// then feed these carried pairs back through [`plan`].
pub fn retained_array_read_options<V>(
    miss: &ArrayReadMiss,
    mut materialise: impl FnMut(&[u8]) -> V,
) -> Vec<CarriedOption<V>> {
    let mut options = vec![(b"-errorcode".to_vec(), materialise(&miss.code))];
    if let Some(info) = &miss.info {
        options.push((b"-errorinfo".to_vec(), materialise(info)));
    }
    if let Some(line) = miss.line {
        options.push((
            b"-errorline".to_vec(),
            materialise(line.to_string().as_bytes()),
        ));
    }
    options
}

/// Plan the complete return-options dictionary for one completion.
///
/// Carried options are retained, including custom keys and explicit error
/// metadata. `-code` and `-level` are always replaced by the settled values;
/// live level-zero error metadata replaces its carried counterpart. A missing
/// `-errorcode` is filled for every error. Synthesis of TIP 348 metadata is
/// gated by the selected Tcl release; a carried pre-TIP option with the same
/// spelling remains an ordinary custom pair.
#[must_use]
pub fn plan<V: Clone>(
    version: TclVersion,
    code: Code,
    level: i64,
    carried: &[CarriedOption<V>],
    error: Option<&ErrorOptions<V>>,
) -> Vec<(Vec<u8>, OptionValue<V>)> {
    let mut rows: Vec<(Vec<u8>, OptionValue<V>)> = carried
        .iter()
        .filter(|(key, _)| key.as_slice() != b"-code" && key.as_slice() != b"-level")
        .map(|(key, value)| (key.clone(), OptionValue::Value(value.clone())))
        .collect();

    rows.push((b"-code".to_vec(), OptionValue::Integer(code.as_int())));
    rows.push((b"-level".to_vec(), OptionValue::Integer(level)));

    if code != Code::Error {
        return rows;
    }
    let Some(error) = error else {
        return rows;
    };
    if version.has_error_stack()
        && let Some(value) = &error.error_stack
    {
        set_or_append(&mut rows, b"-errorstack", OptionValue::Value(value.clone()));
    }
    if let Some(value) = &error.error_code {
        set_or_append(&mut rows, b"-errorcode", OptionValue::Value(value.clone()));
    }
    if let Some(value) = &error.error_info {
        set_or_append(&mut rows, b"-errorinfo", OptionValue::Value(value.clone()));
    }
    if let Some(line) = error.error_line {
        set_or_append(&mut rows, b"-errorline", OptionValue::Integer(line));
    }
    if let Some(value) = &error.during {
        set_or_append(&mut rows, b"-during", OptionValue::Value(value.clone()));
    }
    rows
}

fn set_or_append<V>(rows: &mut Vec<(Vec<u8>, OptionValue<V>)>, key: &[u8], value: OptionValue<V>) {
    if let Some((_, current)) = rows.iter_mut().find(|(candidate, _)| candidate == key) {
        *current = value;
    } else {
        rows.push((key.to_vec(), value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys<'a>(rows: &'a [(Vec<u8>, OptionValue<&'a str>)]) -> Vec<&'a [u8]> {
        rows.iter().map(|(key, _)| key.as_slice()).collect()
    }

    #[test]
    fn control_option_policy_keeps_activation_and_settlement_independent() {
        assert!(!ControlOptionPolicy::TRANSPARENT.begins_fresh());
        assert!(!ControlOptionPolicy::TRANSPARENT.settles_success());
        assert!(ControlOptionPolicy::FRESH_FORWARDED.begins_fresh());
        assert!(!ControlOptionPolicy::FRESH_FORWARDED.settles_success());
        assert!(ControlOptionPolicy::FRESH_SETTLED.begins_fresh());
        assert!(ControlOptionPolicy::FRESH_SETTLED.settles_success());
    }

    #[test]
    fn errors_receive_standard_metadata_and_keep_custom_options() {
        let carried = vec![(b"-foo".to_vec(), "bar")];
        let error = ErrorOptions {
            error_code: Some("NONE"),
            error_info: Some("boom"),
            error_stack: Some("INNER boom"),
            error_line: Some(3),
            during: None,
        };
        let rows = plan(TclVersion::V9_0, Code::Error, 0, &carried, Some(&error));
        assert_eq!(
            keys(&rows),
            [
                b"-foo".as_slice(),
                b"-code",
                b"-level",
                b"-errorstack",
                b"-errorcode",
                b"-errorinfo",
                b"-errorline"
            ]
        );
    }

    #[test]
    fn tip_348_synthesis_is_gated_but_pre_tip_custom_spelling_is_preserved() {
        let carried = vec![
            (b"-errorstack".to_vec(), "explicit"),
            (b"-errorinfo".to_vec(), "INFO"),
        ];
        let error = ErrorOptions {
            error_code: Some("NONE"),
            ..ErrorOptions::default()
        };
        let old = plan(TclVersion::V8_5, Code::Error, 1, &carried, Some(&error));
        assert!(keys(&old).contains(&b"-errorstack".as_slice()));
        let modern = plan(TclVersion::V9_0, Code::Error, 1, &carried, Some(&error));
        assert!(keys(&modern).contains(&b"-errorstack".as_slice()));
        assert!(keys(&modern).contains(&b"-errorinfo".as_slice()));
    }

    #[test]
    fn swallowed_array_reads_carry_only_the_metadata_the_read_created() {
        assert_eq!(
            retained_array_read_options(&ArrayReadMiss::missing(), |bytes| {
                String::from_utf8_lossy(bytes).into_owned()
            }),
            [(b"-errorcode".to_vec(), "TCL READ VARNAME".into())]
        );
        assert_eq!(
            retained_array_read_options(
                &ArrayReadMiss::trace_error(Some(b"boom trace".to_vec()), 3),
                |bytes| String::from_utf8_lossy(bytes).into_owned(),
            ),
            [
                (b"-errorcode".to_vec(), "TCL READ VARNAME".into()),
                (b"-errorinfo".to_vec(), "boom trace".into()),
                (b"-errorline".to_vec(), "3".into()),
            ]
        );
    }
}
