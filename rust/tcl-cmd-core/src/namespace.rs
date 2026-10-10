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

//! `namespace` text-op cores — the pure `::`-qualified-name manipulations
//! (`tail`, `qualifiers`), shared verbatim by both runtimes.
//!
//! The `tail`/`qualifiers` ops are pure byte→byte operations on the *literal*
//! name (no namespace resolution, no runtime state), so each runtime hands in
//! the name bytes and builds its own result value from the returned slice. The
//! `current`/`which` cores *do* read namespace state, so they are generic over
//! the [`Namespaces`] role trait + [`ValueOps`].

use tcl_runtime_api::{Namespaces, NsId};
use tcl_syntax::glob::string_match;
use tcl_syntax::value::ValueOps;

use crate::error::CmdError;

/// Report an original unresolved namespace subcommand through its selected
/// dispatcher. C8.4/8.5 use `Tcl_GetIndexFromObj`'s option grammar; modern C uses
/// its ensemble, while Jim's namespace helper owns its command enumeration.
/// The caller retains the original operand and supplies the available grammar.
#[must_use]
pub fn unknown_subcommand_message(
    protocol: tcl_syntax::naming::NativeNameProtocol,
    available: &[&str],
    original: &[u8],
) -> Vec<u8> {
    use crate::prefix::Resolution;
    use tcl_syntax::naming::NativeNameProtocol;
    let ambiguous = matches!(
        crate::prefix::scan(available, original, false),
        Resolution::Ambiguous
    );
    match protocol {
        NativeNameProtocol::C(version) if version <= tcl_dialect::TclVersion::V8_5 => {
            crate::prefix::bad_key_message(available, b"option", original, ambiguous)
        }
        NativeNameProtocol::C(_) => crate::ensemble::unknown_subcommand_message(
            available,
            original,
            true,
            b"::tcl::namespace",
        ),
        NativeNameProtocol::Jim084 => {
            let mut names = available.to_vec();
            names.sort_unstable();
            let mut message = b"namespace, ".to_vec();
            message.extend_from_slice(if ambiguous { b"ambiguous" } else { b"unknown" });
            message.extend_from_slice(b" command \"");
            message.extend_from_slice(original);
            message.extend_from_slice(b"\": should be ");
            for (index, name) in names.iter().enumerate() {
                if index != 0 {
                    message.extend_from_slice(b", ");
                }
                message.extend_from_slice(name.as_bytes());
            }
            message
        }
    }
}

/// The byte range `(start, end)` of the last `::` separator **run** (two or more
/// consecutive colons) in `s`: `s[..start]` is the qualifier, `s[end..]` the
/// tail. `None` when there is no `::`. Mirrors C Tcl's `TclGetNamespaceForQualName`
/// colon-run handling — a run of 3+ colons is one separator (so `foo:::` has
/// qualifier `foo` and an empty tail), where a naive `rsplit("::")` diverges.
fn last_sep_run(s: &[u8]) -> Option<(usize, usize)> {
    // Scan back for the last "::" pair, then extend over every adjacent colon.
    let mut i = s.len();
    while i >= 2 {
        if s[i - 1] == b':' && s[i - 2] == b':' {
            let mut start = i - 2;
            while start > 0 && s[start - 1] == b':' {
                start -= 1;
            }
            let mut end = i;
            while end < s.len() && s[end] == b':' {
                end += 1;
            }
            return Some((start, end));
        }
        i -= 1;
    }
    None
}

/// The namespace portion of a written qualified name, retaining whether the
/// spelling is rooted.  `namespace qualifiers` intentionally erases this
/// distinction for its public string result, but name resolution cannot: a
/// leading `::` means the global namespace, even when there is no separator
/// before the command tail (`::lassign`).  This is the three-way contract used
/// by `TclGetNamespaceForQualName` in `tclNamesp.c`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Qualifier<'a> {
    /// A rooted name; the slice is the written namespace prefix (`b"::"` for
    /// a global command, `b"::a"` for a command in namespace `a`).
    Absolute(&'a [u8]),
    /// A namespace-relative qualified name (`a::b`).
    Relative(&'a [u8]),
    /// An unqualified name (`b`).
    Unqualified,
}

/// Split a written command/name into its resolution qualifier while retaining
/// the absolute marker.  The public [`qualifiers`] operation below continues
/// to return the historical empty string for `::lassign`.
#[must_use]
pub fn qualifier(name: &[u8]) -> Qualifier<'_> {
    let rooted = name.starts_with(b"::");
    match last_sep_run(name) {
        Some((start, _)) => {
            // The leading root is itself the qualifier when the only `::`
            // run is the rooted marker (`::name`, `::`, or `:::`).
            let prefix = if rooted && start == 0 {
                &name[..2]
            } else {
                &name[..start]
            };
            if rooted {
                Qualifier::Absolute(prefix)
            } else {
                Qualifier::Relative(prefix)
            }
        }
        None if rooted => Qualifier::Absolute(b"::"),
        None => Qualifier::Unqualified,
    }
}

/// `namespace qualifiers string` — everything before the last `::` run (the
/// empty string when `string` is unqualified).
#[must_use]
pub fn qualifiers(name: &[u8]) -> &[u8] {
    match last_sep_run(name) {
        Some((start, _)) => &name[..start],
        None => b"",
    }
}

/// `namespace tail string` — the simple name after the last `::` run (the whole
/// run is skipped, so `foo:::` yields the empty string).
#[must_use]
pub fn tail(name: &[u8]) -> &[u8] {
    match last_sep_run(name) {
        Some((_, end)) => &name[end..],
        None => name,
    }
}

/// Expand a static `namespace import` pattern for a bare command name.
///
/// This is the shared glob decision seam for analyser and LSP consumers:
/// Tcl uses the full `*`/`?`/bracket-class grammar, while the source
/// namespace follows the colon-run qualifier rules above.
#[must_use]
pub fn imported_command_candidate(pattern: &str, name: &str) -> Option<String> {
    let pattern_tail = std::str::from_utf8(tail(pattern.as_bytes())).ok()?;
    if !string_match(pattern_tail, name) {
        return None;
    }
    match qualifier(pattern.as_bytes()) {
        Qualifier::Absolute(ns) | Qualifier::Relative(ns) => {
            let ns = std::str::from_utf8(ns).ok()?;
            Some(tcl_syntax::naming::qualify(ns, name))
        }
        Qualifier::Unqualified => (pattern_tail == name).then(|| name.to_owned()),
    }
}

/// What `namespace which` should resolve its name as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhichKind {
    /// The default / `-command` form.
    Command,
    /// The `-variable` form.
    Variable,
}

/// Parse the words after `namespace which` as C's `NamespaceWhichCmd` does.
///
/// One word is always the name, even when it starts with `-`. Two words are
/// accepted only when the first is an unambiguous abbreviation of `-command`
/// or `-variable`; every other shape is the command's wrong-arity path.
#[must_use]
pub fn which_request<T: AsRef<[u8]>>(args: &[T]) -> Option<(WhichKind, usize)> {
    match args {
        [_] => Some((WhichKind::Command, 0)),
        [option, _] => {
            let option = option.as_ref();
            if option.len() > 1 && b"-command".starts_with(option) {
                Some((WhichKind::Command, 1))
            } else if option.len() > 1 && b"-variable".starts_with(option) {
                Some((WhichKind::Variable, 1))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// A validated `namespace import` pattern: the source namespace and the glob
/// tail matched against its exported command names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportPattern {
    /// The source namespace named by the pattern qualifier.
    pub source: NsId,
    /// The simple command-name glob after the qualifier.
    pub tail: Vec<u8>,
}

/// Why a `namespace import` pattern cannot name a source namespace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportPatternError {
    /// The whole pattern is empty.
    Empty,
    /// The pattern has no namespace qualifier.
    Unqualified(Vec<u8>),
    /// Its qualifier names no namespace.
    Unknown(Vec<u8>),
    /// Its qualifier resolves back to the importing namespace.
    SelfImport {
        /// The written pattern.
        pattern: Vec<u8>,
        /// The source namespace's simple name.
        namespace: Vec<u8>,
    },
}

impl ImportPatternError {
    /// The exact Tcl error result bytes for this failure.
    #[must_use]
    pub fn message(&self) -> Vec<u8> {
        match self {
            Self::Empty => b"empty import pattern".to_vec(),
            Self::Unqualified(pattern) => {
                let mut message = b"no namespace specified in import pattern \"".to_vec();
                message.extend_from_slice(pattern);
                message.push(b'"');
                message
            }
            Self::Unknown(pattern) => {
                let mut message = b"unknown namespace in import pattern \"".to_vec();
                message.extend_from_slice(pattern);
                message.push(b'"');
                message
            }
            Self::SelfImport { pattern, namespace } => {
                let mut message = b"import pattern \"".to_vec();
                message.extend_from_slice(pattern);
                message.extend_from_slice(b"\" tries to import from namespace \"");
                message.extend_from_slice(namespace);
                message.extend_from_slice(b"\" into itself");
                message
            }
        }
    }
}

/// Resolve and validate one `namespace import` pattern against the namespace
/// tree. This is C's `Tcl_Import` source-namespace gate, shared by both runtime
/// adapters before either enumerates or binds commands.
///
/// # Errors
/// Empty and unqualified patterns, unknown source namespaces, and self-imports
/// receive their distinct Tcl diagnostics.
pub fn import_pattern<O: Namespaces + ?Sized>(
    ops: &O,
    destination: NsId,
    pattern: &[u8],
) -> Result<ImportPattern, ImportPatternError> {
    if pattern.is_empty() {
        return Err(ImportPatternError::Empty);
    }
    let qualifier = match qualifier(pattern) {
        Qualifier::Absolute(prefix) | Qualifier::Relative(prefix) => prefix,
        Qualifier::Unqualified => {
            return Err(ImportPatternError::Unqualified(pattern.to_vec()));
        }
    };
    let source = ops
        .find_namespace_bytes(destination, qualifier)
        .filter(|source| ops.namespace_is_live(*source))
        .ok_or_else(|| ImportPatternError::Unknown(pattern.to_vec()))?;
    if source == destination {
        let source_name = ops.name_bytes(source);
        let simple = tail(&source_name);
        return Err(ImportPatternError::SelfImport {
            pattern: pattern.to_vec(),
            namespace: simple.to_vec(),
        });
    }
    Ok(ImportPattern {
        source,
        tail: tail(pattern).to_vec(),
    })
}

/// `namespace current` — the fully-qualified name of the current namespace
/// (`"::"` at the global level).
pub fn current<O: ValueOps + Namespaces>(ops: &mut O) -> O::Value {
    let ns = Namespaces::current(ops);
    let name = ops.name_bytes(ns);
    ops.new_bytes(&name)
}

/// Actual native namespace-result object production, separate from reporting.
pub trait NamespaceObjectBackend: ValueOps + Namespaces {
    /// Produce the original current-namespace result using actual issuer and token.
    ///
    /// # Errors
    /// Refuses unavailable physical production without reparsing reporting bytes.
    fn current_namespace_object(&mut self) -> Result<Self::Value, tcl_syntax::value::ValueError>;

    /// Produce a result from an already selected actual namespace incarnation.
    /// Reporting bytes are never parsed to recover this token.
    ///
    /// # Errors
    /// Refuses an unavailable physical producer or retired namespace lifetime.
    fn produce_namespace_object(
        &mut self,
        _namespace: NsId,
        _producer: tcl_syntax::native_namespace_name::NativeNamespaceObjectProducer,
    ) -> Result<Self::Value, tcl_syntax::value::ValueError> {
        Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "native namespace result producer",
        ))
    }
}

/// Actual namespace retirement, separate from original-object cache lookup.
pub trait NamespaceDeleteBackend: ValueOps + Namespaces {
    /// Retire an already selected namespace through its actual lifecycle owner.
    ///
    /// # Errors
    /// Preserves actual host capability refusals from the retirement callback.
    fn delete_selected_namespace(
        &mut self,
        namespace: NsId,
    ) -> Result<(), tcl_syntax::value::ValueError>;
}

/// Validate every original written name before retiring any namespace.
/// Names are materialised and looked up again after earlier retirements, so
/// callback deletion of another target becomes a harmless second-pass absence.
/// Jim's flat script helper has a separate operation and cannot use this door.
///
/// # Errors
/// Preserves original storage, lookup and retirement refusals. A missing first
/// pass operand is returned by its original index for operation-specific reporting.
pub fn delete_original<O: NamespaceDeleteBackend>(
    ops: &mut O,
    names: &[O::Value],
) -> Result<Option<usize>, tcl_syntax::value::ValueError> {
    use tcl_syntax::value::ValueError;
    let policy = ops
        .name_policy_protocol()
        .ok_or(ValueError::CommandProtocolUnavailable(
            "namespace deletion issuer",
        ))?;
    if policy.recipe().is_jim084() {
        return Err(ValueError::CommandProtocolUnavailable(
            "C namespace retirement in Jim",
        ));
    }
    for (index, original) in names.iter().enumerate() {
        let bytes = ops.native_string_bytes(original)?;
        if ops
            .find_namespace_bytes_checked(Namespaces::current(ops), &bytes)?
            .is_none()
        {
            return Ok(Some(index));
        }
    }
    for original in names {
        let bytes = ops.native_string_bytes(original)?;
        if let Some(namespace) =
            ops.find_namespace_bytes_checked(Namespaces::current(ops), &bytes)?
        {
            ops.delete_selected_namespace(namespace)?;
        }
    }
    Ok(None)
}

/// Produce the physical parent's original result from an already selected token.
///
/// # Errors
/// Preserves selected physical production refusal without name relookup.
pub fn parent_original<O: NamespaceObjectBackend>(
    ops: &mut O,
    namespace: NsId,
) -> Result<O::Value, tcl_syntax::value::ValueError> {
    match ops.parent(namespace) {
        Some(parent) => ops.produce_namespace_object(
            parent,
            tcl_syntax::native_namespace_name::NativeNamespaceObjectProducer::Parent,
        ),
        None => Ok(ops.empty()),
    }
}

/// Produce original child objects from the native ordered and filtered token inventory.
/// Filtering belongs to the independently selected name-query owner.
///
/// # Errors
/// Preserves actual physical production refusal for each reached child.
pub fn children_original<O: NamespaceObjectBackend>(
    ops: &mut O,
    children: &[NsId],
) -> Result<O::Value, tcl_syntax::value::ValueError> {
    namespace_objects_original(
        ops,
        children,
        tcl_syntax::native_namespace_name::NativeNamespaceObjectProducer::Child,
    )
}

/// Produce an ordered list of original objects from selected namespace tokens.
///
/// # Errors
/// Releases earlier producer results before propagating a reached refusal.
pub fn namespace_objects_original<O: NamespaceObjectBackend>(
    ops: &mut O,
    namespaces: &[NsId],
    producer: tcl_syntax::native_namespace_name::NativeNamespaceObjectProducer,
) -> Result<O::Value, tcl_syntax::value::ValueError> {
    let mut items = Vec::with_capacity(namespaces.len());
    for namespace in namespaces {
        match ops.produce_namespace_object(*namespace, producer) {
            Ok(item) => {
                ops.pin_value(&item);
                items.push(item);
            }
            Err(error) => {
                for item in &items {
                    ops.unpin_value(item);
                }
                return Err(error);
            }
        }
    }
    let result = ops.new_list(items.clone());
    for item in &items {
        ops.unpin_value(item);
    }
    Ok(result)
}

/// Select the actual child tokens with the native query's lookup or scan rule.
/// The selected namespace is never recovered from its reporting name.
///
/// # Errors
/// Preserves missing name-policy authority, lookup refusal and matcher refusal.
pub fn children_tokens_checked<O: ValueOps + Namespaces>(
    ops: &O,
    namespace: NsId,
    pattern: Option<&[u8]>,
) -> Result<Vec<NsId>, tcl_syntax::value::ValueError> {
    use tcl_syntax::{
        native_glob::{
            NativeGlobProtocol, NativeNameGlobPurpose, NativeNamespaceChildrenLookup,
            namespace_children_lookup,
        },
        value::ValueError,
    };
    let policy = ops
        .name_policy_protocol()
        .ok_or(ValueError::CommandProtocolUnavailable(
            "namespace children name policy",
        ))?;
    let target = ops.name_bytes(namespace);
    let qualified = pattern.map(|pattern| {
        if pattern.starts_with(b"::") {
            pattern.to_vec()
        } else {
            let mut qualified = target.clone();
            if qualified != b"::" {
                qualified.extend_from_slice(b"::");
            }
            qualified.extend_from_slice(pattern);
            qualified
        }
    });
    let purpose = NativeNameGlobPurpose::NamespaceChildrenSearch;
    if let Some(pattern) = qualified.as_deref() {
        match namespace_children_lookup(policy.recipe(), &target, pattern) {
            NativeNamespaceChildrenLookup::FullName(name) => {
                return Ok(ops
                    .find_namespace_bytes_checked(namespace, name)?
                    .filter(|child| ops.parent(*child) == Some(namespace))
                    .into_iter()
                    .collect());
            }
            NativeNamespaceChildrenLookup::ChildKey(key) => {
                return Ok(match key {
                    Some(key) => ops.find_namespace_child_bytes_checked(namespace, key)?,
                    None => None,
                }
                .into_iter()
                .collect());
            }
            NativeNamespaceChildrenLookup::Scan => {}
        }
    }
    let matcher = NativeGlobProtocol::from_name_policy(policy);
    let mut children = Vec::new();
    for child in ops.children_hash_order(namespace) {
        let keep = match qualified.as_deref() {
            Some(pattern) => matcher
                .match_name_pattern(purpose, pattern, &ops.name_bytes(child))
                .map_err(|_| {
                    ValueError::CommandProtocolUnavailable("namespace children native match")
                })?,
            None => true,
        };
        if keep {
            children.push(child);
        }
    }
    Ok(children)
}

/// Invoke the genuine current-namespace producer instead of its reporting factory.
///
/// # Errors
/// Preserves the actual object producer's host capability refusal.
pub fn current_original<O: NamespaceObjectBackend>(ops: &mut O) -> Result<O::Value, CmdError> {
    Ok(ops.current_namespace_object()?)
}

/// Validate actual original arguments before current-namespace object production.
///
/// # Errors
/// Reports native wrong arguments or preserves the selected physical refusal.
pub fn current_original_with_arguments<O: NamespaceObjectBackend>(
    ops: &mut O,
    args: &[O::Value],
    usage: &[u8],
) -> Result<O::Value, CmdError> {
    if !args.is_empty() {
        return Err(CmdError::wrong_args_bytes(usage));
    }
    current_original(ops)
}

/// Invoke the selected current-namespace handler with its actual usage header.
/// The shared adapter supplies the retained ensemble/alias rewrite, independently
/// of the worker's implementation spelling.
///
/// # Errors
/// Reports native arity validation before reading namespace state.
pub fn current_with_arguments<O: ValueOps + Namespaces>(
    ops: &mut O,
    args: &[O::Value],
    usage: &[u8],
) -> Result<O::Value, CmdError> {
    if !args.is_empty() {
        return Err(CmdError::wrong_args_bytes(usage));
    }
    Ok(current(ops))
}

/// `namespace which -command name` — the fully-qualified name `name` resolves to
/// as a command from the current namespace, or the empty string if it resolves
/// to nothing. (Option parsing and the `-variable` form stay in each adapter.)
pub fn which_command<O: ValueOps + Namespaces>(ops: &mut O, name: &str) -> O::Value {
    let cur = Namespaces::current(ops);
    match ops
        .find_command(cur, name)
        .and_then(|id| ops.command_name(id))
    {
        Some(fqn) => ops.new_string(fqn),
        None => ops.empty(),
    }
}

/// [`which_command`] over a **byte-valued** name, as the resolved FQN bytes —
/// the form a byte-native runtime uses so a command name survives verbatim.
pub fn which_command_bytes<O: Namespaces + ?Sized>(ops: &O, name: &[u8]) -> Option<Vec<u8>> {
    let cur = ops.current();
    ops.find_command_bytes(cur, name)
        .and_then(|id| ops.command_name_bytes(id))
}

/// Checked byte command query; an unavailable operation is not a missing name.
///
/// # Errors
/// Propagates the actual namespace adapter's native lookup refusal.
pub fn which_command_bytes_checked<O: Namespaces + ?Sized>(
    ops: &O,
    name: &[u8],
) -> Result<Option<Vec<u8>>, tcl_syntax::value::ValueError> {
    match ops.find_command_bytes_checked(ops.current(), name)? {
        Some(id) => command_name_from_command_checked(ops, id).map(Some),
        None => Ok(None),
    }
}

/// Report the current full name of an already selected command token. Imports
/// keep their own names here; origin reporting is a separate query.
///
/// # Errors
/// Refuses when the actual selected token cannot report its current name.
pub fn command_name_from_command_checked<O: Namespaces + ?Sized>(
    ops: &O,
    command: tcl_runtime_api::CommandId,
) -> Result<Vec<u8>, tcl_syntax::value::ValueError> {
    ops.command_name_bytes(command).ok_or(
        tcl_syntax::value::ValueError::CommandProtocolUnavailable("command name reporting"),
    )
}

/// Checked native namespace-variable query, independently of scalar access.
///
/// # Errors
/// Propagates the adapter's unavailable actual query protocol.
pub fn which_variable_bytes_checked<O: Namespaces + ?Sized>(
    ops: &O,
    name: &[u8],
) -> Result<Option<Vec<u8>>, tcl_syntax::value::ValueError> {
    ops.namespace_variable_name_bytes_checked(ops.current(), name)
}

/// `namespace which -variable name` — the fully-qualified name `name` resolves
/// to as a **namespace** variable from the current namespace, or the empty
/// string.
///
/// C's `NamespaceWhichCmd` case 1 calls `Tcl_FindNamespaceVar(interp, name,
/// NULL, 0)` (`tclNamesp.c:4657-4664`), which walks namespace `varTable`s and
/// **never** the call frame: a proc-local variable answers the empty string
/// even while it is set, and a qualified name commits to the namespace its
/// qualifier resolves to.
///
/// The one release axis is the *alternate* search. `ObjFindNamespaceVar`
/// (`tclVar.c`) hands `TclGetNamespaceForQualName` two candidate namespaces
/// and tries both; Tcl 9.0 added `flags |= TCL_NAMESPACE_ONLY`
/// (`tcl9.0.4 tclVar.c:5951-5953`), which blanks the second, so 8.x falls back
/// to the **global** namespace where 9.0 does not — tclsh-pinned:
/// `set ::gv 1; namespace eval n {namespace which -variable gv}` is `::gv` on
/// 8.6.16 and `{}` on 9.0.4.
pub fn which_variable<O: ValueOps + Namespaces>(
    ops: &mut O,
    name: &str,
    profile: &tcl_dialect::DialectProfile,
) -> O::Value {
    match variable_fqn(ops, name, profile) {
        Some(fqn) => ops.new_string(fqn),
        None => ops.empty(),
    }
}

/// [`which_variable`]'s resolution step over a **byte-valued** name — the
/// `&self` form for adapters that hold names as bytes.
///
/// A Tcl variable name is a byte string (`set [binary format c 255] 1`), so
/// the resolution runs on bytes end to end: the qualifier split is already a
/// byte op, and the namespace/table probes go through the `Namespaces`
/// byte-valued spellings. A byte-native runtime therefore never has to route
/// a name through `str`.
pub fn variable_fqn_bytes<O: Namespaces + ?Sized>(
    ops: &O,
    name: &[u8],
    profile: &tcl_dialect::DialectProfile,
) -> Option<Vec<u8>> {
    let cur = ops.current();
    // Variable resolution has no existence-checked fall-through the way
    // command resolution does (`tclVar.c`'s `TclLookupSimpleVar`): the
    // qualifier names the namespace outright.
    let (primary, simple): (Option<tcl_runtime_api::NsId>, Vec<u8>) = match qualifier(name) {
        Qualifier::Unqualified => (Some(cur), name.to_vec()),
        Qualifier::Absolute(prefix) | Qualifier::Relative(prefix) => {
            (ops.find_namespace_bytes(cur, prefix), tail(name).to_vec())
        }
    };
    // The alternate (global-rooted) candidate — TIP 278, dropped from 9.0 on.
    // The release test is the dialect layer's to own (it follows the vendor
    // shells' embedded core version, which a bare `TCL90_PLUS` mask test gets
    // wrong), so ask it rather than re-deriving the comparison here. An
    // absolute name has no alternate in any release: it already names the
    // global-rooted namespace.
    let alternate = match qualifier(name) {
        _ if !profile.namespace_var_global_fallback() => None,
        Qualifier::Absolute(_) => None,
        Qualifier::Unqualified => ops.find_namespace_bytes(cur, b"::"),
        Qualifier::Relative(prefix) => {
            let mut rooted = b"::".to_vec();
            rooted.extend_from_slice(prefix);
            ops.find_namespace_bytes(cur, &rooted)
        }
    };
    let ns = [primary, alternate]
        .into_iter()
        .flatten()
        .find(|ns| ops.namespace_var_exists_bytes(*ns, &simple))?;
    let mut fqn = ops.name_bytes(ns);
    if fqn != b"::" {
        fqn.extend_from_slice(b"::");
    }
    fqn.extend_from_slice(&simple);
    Some(fqn)
}

/// [`variable_fqn_bytes`] for the UTF-8-keyed adapters (the VM), whose own
/// tables cannot hold a non-UTF-8 name in the first place.
pub fn variable_fqn<O: Namespaces + ?Sized>(
    ops: &O,
    name: &str,
    profile: &tcl_dialect::DialectProfile,
) -> Option<String> {
    variable_fqn_bytes(ops, name.as_bytes(), profile)
        .map(|fqn| String::from_utf8_lossy(&fqn).into_owned())
}

/// `namespace origin command` over a **byte-valued** name — the fully-qualified
/// name of the *original* command `name` resolves to, following `namespace
/// import` links to their source. This is `NamespaceOriginCmd`
/// (`tclNamesp.c`): resolve the word to a command token, ask
/// `TclGetOriginalCommand` for the token it was imported from (falling back to
/// the token itself when it was not imported), and report that token's
/// fully-qualified name. `None` when the word resolves to no command at all —
/// the caller reports `invalid command name "<name>"`.
pub fn origin_bytes<O: Namespaces + ?Sized>(ops: &O, name: &[u8]) -> Option<Vec<u8>> {
    let cur = ops.current();
    let mut cmd = ops.find_command_bytes(cur, name)?;
    if ops.namespace_import_binding() == Some(tcl_dialect::NamespaceImportBinding::SourceName) {
        let root = ops.root_command_context_checked().ok()??;
        let mut visited = std::collections::HashSet::new();
        while let Some(prefix) = ops.command_alias_prefix_bytes(cmd) {
            if !visited.insert(cmd) {
                return None;
            }
            // Jim's namespace helper passes the entire `info alias` result
            // as one command-name word, including any prefix arguments.
            let target = if let [target] = prefix.as_slice() {
                target.clone()
            } else {
                let mut list = Vec::new();
                for (index, word) in prefix.iter().enumerate() {
                    tcl_syntax::list::append_list_element(&mut list, word, index == 0);
                }
                list
            };
            cmd = ops.find_command_bytes(root, &target)?;
        }
        return ops.command_name_bytes(cmd);
    }
    ops.command_name_bytes(ops.command_origin(cmd).unwrap_or(cmd))
}

/// Checked original-token query, retaining native byte aliases and refusals.
///
/// # Errors
/// Propagates checked command/namespace lookup failures at each alias hop.
pub fn origin_bytes_checked<O: Namespaces + ?Sized>(
    ops: &O,
    name: &[u8],
) -> Result<Option<Vec<u8>>, tcl_syntax::value::ValueError> {
    let Some(mut command) = ops.find_command_bytes_checked(ops.current(), name)? else {
        return Ok(None);
    };
    if ops.namespace_import_binding() == Some(tcl_dialect::NamespaceImportBinding::SourceName) {
        let Some(root) = ops.root_command_context_checked()? else {
            return Ok(None);
        };
        let mut visited = std::collections::HashSet::new();
        while let Some(prefix) = ops.command_alias_prefix_bytes_checked(command)? {
            if !visited.insert(command) {
                return Ok(None);
            }
            let target = if let [target] = prefix.as_slice() {
                target.clone()
            } else {
                let mut list = Vec::new();
                for (index, word) in prefix.iter().enumerate() {
                    tcl_syntax::list::append_list_element(&mut list, word, index == 0);
                }
                list
            };
            let Some(target) = ops.find_command_bytes_checked(root, &target)? else {
                return Ok(None);
            };
            command = target;
        }
        return ops.command_name_bytes(command).map(Some).ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("command origin reporting"),
        );
    }
    origin_from_command_checked(ops, command).map(Some)
}

/// Report the original command from an already resolved, retained command token.
/// This performs no command-name lookup or original-object String conversion.
///
/// # Errors
/// Refuses when the selected original token cannot report its current full name.
pub fn origin_from_command_checked<O: Namespaces + ?Sized>(
    ops: &O,
    command: tcl_runtime_api::CommandId,
) -> Result<Vec<u8>, tcl_syntax::value::ValueError> {
    ops.command_name_bytes(ops.command_origin(command).unwrap_or(command))
        .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "command origin reporting",
        ))
}

/// [`origin_bytes`] for the UTF-8-keyed adapters (the VM).
pub fn origin<O: Namespaces + ?Sized>(ops: &O, name: &str) -> Option<String> {
    origin_bytes(ops, name.as_bytes()).map(|fqn| String::from_utf8_lossy(&fqn).into_owned())
}

/// A byte-valued namespace target that did not resolve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamespaceLookupError {
    name: Vec<u8>,
}

impl NamespaceLookupError {
    /// The byte-exact namespace spelling supplied by the caller.
    #[must_use]
    pub fn name(&self) -> &[u8] {
        &self.name
    }

    /// Tcl's byte-exact `namespace "<name>" not found` result.
    #[must_use]
    pub fn message(&self) -> Vec<u8> {
        let mut message = b"namespace \"".to_vec();
        message.extend_from_slice(&self.name);
        message.extend_from_slice(b"\" not found");
        message
    }
}

/// `namespace exists name` over a byte-valued namespace name.
pub fn exists_bytes<O: ValueOps + Namespaces>(ops: &mut O, name: &[u8]) -> O::Value {
    let cur = Namespaces::current(ops);
    let present = ops
        .find_namespace_bytes(cur, name)
        .is_some_and(|ns| ops.namespace_is_live(ns));
    ops.new_bool(present)
}

/// [`exists_bytes`] for UTF-8-keyed adapters.
pub fn exists<O: ValueOps + Namespaces>(ops: &mut O, name: &str) -> O::Value {
    exists_bytes(ops, name.as_bytes())
}

/// `namespace parent ?name?` over a byte-valued namespace name.
///
/// # Errors
/// `namespace "<name>" not found` if `name` is given and does not resolve.
pub fn parent_bytes<O: ValueOps + Namespaces>(
    ops: &mut O,
    name: Option<&[u8]>,
) -> Result<O::Value, NamespaceLookupError> {
    let ns = resolve_target_bytes(ops, name)?;
    let fqn = match ops.parent(ns) {
        Some(parent) => ops.name_bytes(parent),
        None => Vec::new(), // the global root has no parent
    };
    Ok(ops.new_bytes(&fqn))
}

/// `namespace parent ?name?` — the FQN of the (named, or current) namespace's
/// parent (the global root's parent is the empty string).
///
/// # Errors
/// `namespace "<name>" not found` if `name` is given and does not resolve.
pub fn parent<O: ValueOps + Namespaces>(
    ops: &mut O,
    name: Option<&str>,
) -> Result<O::Value, CmdError> {
    parent_bytes(ops, name.map(str::as_bytes))
        .map_err(|error| CmdError::new(String::from_utf8_lossy(&error.message()).into_owned()))
}

/// `namespace children ?name? ?pattern?` over byte-valued namespace names.
///
/// The shared token-query owner selects the release's exact child-table lookup
/// or opaque-byte glob scan. This reporting adapter constructs byte-valued names
/// only after the original namespace tokens have been selected.
///
/// # Errors
/// Preserves missing namespaces and unavailable native lookup or matcher policy.
pub fn children_bytes<O: ValueOps + Namespaces>(
    ops: &mut O,
    name: Option<&[u8]>,
    pattern: Option<&[u8]>,
) -> Result<O::Value, CmdError> {
    let ns =
        resolve_target_bytes(ops, name).map_err(|error| CmdError::new_bytes(error.message()))?;
    let tokens = children_tokens_checked(ops, ns, pattern)?;
    let names: Vec<_> = tokens
        .into_iter()
        .map(|child| ops.name_bytes(child))
        .collect();
    let items = names.iter().map(|name| ops.new_bytes(name)).collect();
    Ok(ops.new_list(items))
}

/// `namespace children ?name? ?pattern?` — the FQNs of the (named, or current)
/// namespace's child namespaces, glob-filtered in C's hash-table iteration
/// order. A pattern without a
/// leading `::` is qualified with the target namespace's FQN first (C's
/// `NamespaceChildrenCmd`).
///
/// # Errors
/// `namespace "<name>" not found` if `name` is given and does not resolve.
pub fn children<O: ValueOps + Namespaces>(
    ops: &mut O,
    name: Option<&str>,
    pattern: Option<&str>,
) -> Result<O::Value, CmdError> {
    children_bytes(ops, name.map(str::as_bytes), pattern.map(str::as_bytes))
}

/// The observable order owner for Tcl's `TCL_STRING_KEYS` hash table.
///
/// Namespace adapters keep one instance per namespace child table *and* one per
/// namespace command table instead of reconstructing either from the live
/// entries: Tcl quadruples the bucket array at a 3:1 load factor and never
/// shrinks it when entries are deleted, while each resize also reverses bucket
/// chains. Both facts affect `Tcl_FirstHashEntry` order, which
/// `TclDeleteNamespaceChildren` and `TclTeardownNamespace` snapshot before
/// deleting each token.
#[derive(Clone, Debug)]
pub struct TclStringHashOrder {
    order: tcl_core_types::NativeHashOrder,
}

impl Default for TclStringHashOrder {
    fn default() -> Self {
        // The legacy pure helper preserves its unsigned host-word arithmetic.
        // This convenience constructor does not issue native ABI authority.
        let width = if usize::BITS == 32 {
            tcl_core_types::NativeHashWordWidth::Bits32
        } else {
            tcl_core_types::NativeHashWordWidth::Bits64
        };
        Self::with_recipe(tcl_core_types::NativeHashRecipe::Tcl {
            promotion: tcl_core_types::NativeHashBytePromotion::Unsigned,
            width,
        })
    }
}

impl TclStringHashOrder {
    /// Use an independently selected ABI-aware recipe with the shared chain kernel.
    #[must_use]
    pub fn with_recipe(recipe: tcl_core_types::NativeHashRecipe) -> Self {
        Self {
            order: tcl_core_types::NativeHashOrder::new(recipe),
        }
    }

    /// Insert a physical entry, returning false when it already exists.
    pub fn insert(&mut self, key: &[u8]) -> bool {
        self.order.insert(key)
    }

    /// Re-create `key`'s entry at its bucket head, as C's
    /// `TclCreateObjCommandInNs` does when it redefines an existing command:
    /// the old hash entry is deleted and a fresh one created, which moves the
    /// name to the front of its chain.
    pub fn reinsert(&mut self, key: &[u8]) {
        self.remove(key);
        self.insert(key);
    }

    /// Live entry count (`Tcl_HashTable.numEntries`).
    #[must_use]
    pub fn len(&self) -> usize {
        self.order.len()
    }

    /// Whether the table holds no live entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    /// Retain at least `bucket_count` buckets without adding entries.
    ///
    /// Dictionary value adapters use this when copying a Tcl hash table: the
    /// bucket array grows but never shrinks, so rebuilding only from the live
    /// entries would lose observable `dict info` history after removals.
    pub fn retain_bucket_count(&mut self, bucket_count: usize) {
        self.order.retain_bucket_count(bucket_count);
    }

    /// Current bucket-array size (`Tcl_HashTable.numBuckets`).
    #[must_use]
    pub fn bucket_count(&self) -> usize {
        self.order.bucket_count()
    }

    /// Delete `key` without shrinking or rebuilding the bucket array.
    pub fn remove(&mut self, key: &[u8]) -> bool {
        self.order.remove(key)
    }

    /// Reset a deleted namespace's table to Tcl's four static buckets.
    pub fn clear(&mut self) {
        self.order = tcl_core_types::NativeHashOrder::new(self.order.recipe());
    }

    /// Live keys in `Tcl_FirstHashEntry`/`Tcl_NextHashEntry` order.
    #[must_use]
    pub fn keys(&self) -> Vec<&[u8]> {
        self.order.keys()
    }

    /// Render Tcl's human-readable hash-table statistics.
    ///
    /// This is the portable owner for `Tcl_HashStats`: `dict info` and any
    /// other consumer of a Tcl-shaped hash table must report the same bucket
    /// histogram and average search distance rather than reconstructing the
    /// table layout or formatting in its runtime adapter.
    #[must_use]
    pub fn statistics(&self) -> String {
        use std::fmt::Write as _;

        const COUNTERS: usize = 10;
        let mut counts = [0_usize; COUNTERS];
        let mut overflow = 0_usize;
        let mut average = 0.0;
        let total_entries = self
            .order
            .len()
            .to_string()
            .parse::<f64>()
            .expect("usize decimal representation is a finite f64");

        for entries in self.order.bucket_lengths() {
            if entries < COUNTERS {
                counts[entries] += 1;
            } else {
                overflow += 1;
            }
            if !self.order.is_empty() {
                // Tcl_HashStats accumulates one `double` contribution per
                // bucket before formatting with `%.1f`. Preserve that exact
                // operation order: an exact rational calculation differs at
                // decimal ties because the preceding additions have already
                // rounded in binary floating point.
                let entries = entries
                    .to_string()
                    .parse::<f64>()
                    .expect("usize decimal representation is a finite f64");
                average += (entries + 1.0) * (entries / total_entries) / 2.0;
            }
        }

        let mut result = format!(
            "{} entries in table, {} buckets\n",
            self.order.len(),
            self.order.bucket_count()
        );
        for (entries, count) in counts.into_iter().enumerate() {
            writeln!(result, "number of buckets with {entries} entries: {count}")
                .expect("writing to a String cannot fail");
        }
        writeln!(
            result,
            "number of buckets with {COUNTERS} or more entries: {overflow}"
        )
        .expect("writing to a String cannot fail");
        write!(result, "average search distance for entry: {average:.1}")
            .expect("writing to a String cannot fail");
        result
    }
}

/// Resolve the optional `name` argument to a namespace handle: the current
/// namespace when absent, else `name` resolved from it (erroring if it doesn't
/// exist) — the shared first step of `namespace parent`/`children`.
fn resolve_target_bytes<O: Namespaces + ?Sized>(
    ops: &O,
    name: Option<&[u8]>,
) -> Result<tcl_runtime_api::NsId, NamespaceLookupError> {
    let cur = Namespaces::current(ops);
    match name {
        None => Ok(cur),
        Some(name) => ops
            .find_namespace_bytes(cur, name)
            .filter(|ns| ops.namespace_is_live(*ns))
            .ok_or_else(|| NamespaceLookupError {
                name: name.to_vec(),
            }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qualifiers_and_tail_match_c() {
        assert_eq!(qualifiers(b"::foo::bar"), b"::foo");
        assert_eq!(tail(b"::foo::bar"), b"bar");
        // Unqualified: no qualifier, the whole name is the tail.
        assert_eq!(qualifiers(b"bar"), b"");
        assert_eq!(tail(b"bar"), b"bar");
        // Colon runs (3+) are a single separator (C semantics; the naive
        // `rsplit("::")` the VM used yielded `:` for the tail here).
        assert_eq!(qualifiers(b"foo:::"), b"foo");
        assert_eq!(tail(b"foo:::"), b"");
        assert_eq!(qualifiers(b"a::::b"), b"a");
        assert_eq!(tail(b"a::::b"), b"b");
        // A trailing simple separator.
        assert_eq!(qualifiers(b"::foo::"), b"::foo");
        assert_eq!(tail(b"::foo::"), b"");
        assert_eq!(qualifier(b"::lassign"), Qualifier::Absolute(b"::"));
        assert_eq!(qualifier(b"a:::b"), Qualifier::Relative(b"a"));
        assert_eq!(qualifier(b"::a:::b"), Qualifier::Absolute(b"::a"));
        assert_eq!(qualifier(b"foo:::"), Qualifier::Relative(b"foo"));
        assert_eq!(qualifier(b"b"), Qualifier::Unqualified);
    }

    #[test]
    fn imported_command_candidate_uses_tcl_globs_and_colon_runs() {
        assert_eq!(
            imported_command_candidate("::src:::im*", "image"),
            Some("::src::image".to_owned())
        );
        assert_eq!(
            imported_command_candidate("::src::he*r", "helper"),
            Some("::src::helper".to_owned())
        );
        assert_eq!(
            imported_command_candidate("::src::helpe?", "helper"),
            Some("::src::helper".to_owned())
        );
        assert_eq!(
            imported_command_candidate("::src::[lp]*", "lookup"),
            Some("::src::lookup".to_owned())
        );
        assert_eq!(
            imported_command_candidate("foo:::", ""),
            Some("::foo::".to_owned())
        );
        assert_eq!(imported_command_candidate("::src::im*", "other"), None);
    }

    #[test]
    fn which_request_matches_namespace_whichs_positional_option_rule() {
        assert_eq!(which_request(&["puts"]), Some((WhichKind::Command, 0)));
        assert_eq!(which_request(&["-command"]), Some((WhichKind::Command, 0)));
        assert_eq!(
            which_request(&["-com", "puts"]),
            Some((WhichKind::Command, 1))
        );
        assert_eq!(
            which_request(&["-var", "name"]),
            Some((WhichKind::Variable, 1))
        );
        assert_eq!(which_request::<&str>(&[]), None);
        assert_eq!(which_request(&["-zork", "puts"]), None);
        assert_eq!(which_request(&["-command", "-variable", "puts"]), None);
    }

    #[test]
    fn tcl_string_hash_order_matches_namespace_children_oracle() {
        let names = [
            "::order::one",
            "::order::two",
            "::order::three",
            "::order::four",
            "::order::five",
            "::order::six",
            "::order::seven",
            "::order::eight",
            "::order::nine",
            "::order::ten",
        ];
        let mut table = TclStringHashOrder::default();
        for name in names {
            assert!(table.insert(tail(name.as_bytes())));
        }
        let actual: Vec<_> = table
            .keys()
            .into_iter()
            .map(|key| core::str::from_utf8(key).unwrap())
            .collect();
        assert_eq!(
            actual,
            [
                "six", "four", "three", "eight", "seven", "nine", "five", "two", "one", "ten",
            ]
        );
    }

    #[test]
    fn tcl_string_hash_order_matches_command_growth_oracles() {
        // TclTeardownNamespace snapshots cmdTable in Tcl_FirstHashEntry order,
        // so a namespace's command-delete traces expose every rebuild the
        // table went through. Exact tclsh 9.0.4 oracle results (identical on
        // 8.6.16).
        let order_of = |count: usize, prefix: &str| -> Vec<String> {
            let mut table = TclStringHashOrder::default();
            for index in 0..count {
                assert!(table.insert(format!("{prefix}{index}").as_bytes()));
            }
            assert_eq!(table.len(), count);
            table
                .keys()
                .into_iter()
                .map(|key| String::from_utf8(key.to_vec()).unwrap())
                .collect()
        };
        // 13 entries cross the 12-entry threshold: 4 buckets become 16.
        assert_eq!(
            order_of(13, "c"),
            [
                "c5", "c6", "c7", "c8", "c9", "c0", "c1", "c10", "c2", "c11", "c12", "c3", "c4",
            ]
        );
        // 49 entries cross the 48-entry threshold too: 16 buckets become 64.
        let mut expected: Vec<String> = (10..49).map(|index| format!("k{index}")).collect();
        expected.extend((0..10).map(|index| format!("k{index}")));
        assert_eq!(order_of(49, "k"), expected);
    }

    #[test]
    fn tcl_string_hash_order_reinsert_moves_the_key_to_its_bucket_head() {
        // Redefining a command deletes its hash entry and creates a new one
        // (TclCreateObjCommandInNs), so the name moves to the chain head.
        // Exact tclsh 9.0.4 oracle result for `proc ::N::one` redefined after
        // the other nine.
        let names = [
            "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
        ];
        let mut table = TclStringHashOrder::default();
        for name in names {
            assert!(table.insert(name.as_bytes()));
        }
        table.reinsert(b"one");
        assert_eq!(table.len(), names.len());
        let actual: Vec<_> = table
            .keys()
            .into_iter()
            .map(|key| core::str::from_utf8(key).unwrap())
            .collect();
        assert_eq!(
            actual,
            [
                "six", "four", "three", "eight", "seven", "one", "nine", "five", "two", "ten",
            ]
        );
        // A plain insert of a live key is a no-op: C finds the entry and never
        // relocates it.
        assert!(!table.insert(b"one"));
        assert_eq!(
            table.keys().first().copied(),
            Some(b"six".as_slice()),
            "insert must not move a live key"
        );
    }

    #[test]
    fn tcl_string_hash_order_retains_resize_after_deletion() {
        let mut table = TclStringHashOrder::default();
        for index in 0..12 {
            assert!(table.insert(format!("a{index}").as_bytes()));
        }
        for index in [1, 2, 4, 5, 6, 7, 8, 9, 10, 11] {
            assert!(table.remove(format!("a{index}").as_bytes()));
        }
        assert_eq!(table.keys(), [b"a0".as_slice(), b"a3".as_slice()]);
        assert_eq!(table.len(), 2);
        // Fresh entries land in the retained 16-bucket array, so they precede
        // the survivors rather than interleaving with them. Exact tclsh 9.0.4
        // oracle result for the same sequence of `proc`/`rename` commands.
        for name in ["b1", "b2", "b3"] {
            assert!(table.insert(name.as_bytes()));
        }
        assert_eq!(
            table.keys(),
            [
                b"b1".as_slice(),
                b"b2".as_slice(),
                b"b3".as_slice(),
                b"a0".as_slice(),
                b"a3".as_slice(),
            ]
        );
    }

    #[test]
    fn tcl_hash_statistics_match_dict_info_oracles() {
        let empty = TclStringHashOrder::default();
        assert_eq!(
            empty.statistics(),
            "0 entries in table, 4 buckets\n\
             number of buckets with 0 entries: 4\n\
             number of buckets with 1 entries: 0\n\
             number of buckets with 2 entries: 0\n\
             number of buckets with 3 entries: 0\n\
             number of buckets with 4 entries: 0\n\
             number of buckets with 5 entries: 0\n\
             number of buckets with 6 entries: 0\n\
             number of buckets with 7 entries: 0\n\
             number of buckets with 8 entries: 0\n\
             number of buckets with 9 entries: 0\n\
             number of buckets with 10 or more entries: 0\n\
             average search distance for entry: 0.0"
        );

        let mut grown = TclStringHashOrder::default();
        for index in 0..13 {
            assert!(grown.insert(format!("k{index}").as_bytes()));
        }
        assert_eq!(
            grown.statistics(),
            "13 entries in table, 16 buckets\n\
             number of buckets with 0 entries: 6\n\
             number of buckets with 1 entries: 7\n\
             number of buckets with 2 entries: 3\n\
             number of buckets with 3 entries: 0\n\
             number of buckets with 4 entries: 0\n\
             number of buckets with 5 entries: 0\n\
             number of buckets with 6 entries: 0\n\
             number of buckets with 7 entries: 0\n\
             number of buckets with 8 entries: 0\n\
             number of buckets with 9 entries: 0\n\
             number of buckets with 10 or more entries: 0\n\
             average search distance for entry: 1.2"
        );

        // Tcl 9.0.4 oracle: 57 occupied buckets, with three deliberate
        // one-byte hash collisions. The exact search distance is 1.05, but
        // Tcl's per-bucket double accumulation lands just above the tie and
        // `%.1f` therefore prints 1.1 (an exact rational implementation
        // incorrectly printed 1.0).
        let mut floating_tie = TclStringHashOrder::default();
        for byte in (33_u8..=89).chain(97..=99) {
            assert!(floating_tie.insert(&[byte]));
        }
        assert_eq!(
            floating_tie.statistics(),
            "60 entries in table, 64 buckets\n\
             number of buckets with 0 entries: 7\n\
             number of buckets with 1 entries: 54\n\
             number of buckets with 2 entries: 3\n\
             number of buckets with 3 entries: 0\n\
             number of buckets with 4 entries: 0\n\
             number of buckets with 5 entries: 0\n\
             number of buckets with 6 entries: 0\n\
             number of buckets with 7 entries: 0\n\
             number of buckets with 8 entries: 0\n\
             number of buckets with 9 entries: 0\n\
             number of buckets with 10 or more entries: 0\n\
             average search distance for entry: 1.1"
        );
    }
}

#[cfg(test)]
mod native_dispatch_diagnostic_tests {
    #[test]
    fn original_namespace_dispatch_errors_match_all_eighteen_native_results() {
        // Native proof naming.namespace.dispatch-error-noun-and-ambiguity:
        // docs/design/analysis/name-resolution-proofs/namespace-dispatch-error-noun-and-ambiguity.md
        // Native proof naming.namespace.dispatch-prefix-worker-availability:
        // docs/design/analysis/name-resolution-proofs/namespace-dispatch-prefix-worker-availability.md
        use tcl_syntax::naming::NativeNameProtocol;
        let mut compared = 0;
        for row in include_str!("../tests/data/native_namespace_dispatch/errors.tsv").lines() {
            let fields: Vec<_> = row.split('\t').collect();
            let available = include_str!("../tests/data/native_namespace_dispatch/available.tsv")
                .lines()
                .find_map(|line| {
                    let (engine, words) = line.split_once('\t')?;
                    (engine == fields[0]).then(|| words.split(' ').collect::<Vec<_>>())
                })
                .expect("same native engine's captured available command words");
            let protocol = if fields[0] == "jim" {
                NativeNameProtocol::Jim084
            } else {
                NativeNameProtocol::C(
                    tcl_dialect::TclVersion::from_dialect(Some(&format!("tcl{}", fields[0])))
                        .expect("captured native C release"),
                )
            };
            let expected: Vec<_> = fields[2]
                .as_bytes()
                .as_chunks::<2>()
                .0
                .iter()
                .map(|digits| u8::from_str_radix(std::str::from_utf8(digits).unwrap(), 16).unwrap())
                .collect();
            assert_eq!(
                super::unknown_subcommand_message(protocol, &available, fields[1].as_bytes()),
                expected,
                "{row}"
            );
            compared += 1;
        }
        assert_eq!(compared, 18);
    }
}
