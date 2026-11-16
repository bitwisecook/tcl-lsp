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

//! The namespace tree and the command lookup owners: `Namespaces`.

use super::commands::NativeJimCommandInventoryKind;
use super::native_execution_error::NativeExecutionError;
use tcl_core_types::{CommandId, NsId};
use tcl_syntax::naming::NativeNamespaceReportParts;

/// The namespace tree and actual command lookup owners.
pub trait Namespaces {
    /// Full failure of an original name query and its retained command owner.
    type OriginalNameError: From<tcl_syntax::value::ValueError> + From<NativeExecutionError>;
    /// Whether imports retain source tokens or resolve source names on use.
    fn namespace_import_binding(&self) -> Option<tcl_dialect::NamespaceImportBinding> {
        None
    }

    /// Project an actual retained alias prefix, preserving conversion refusal.
    /// A reporting name cannot select a different command generation.
    ///
    /// # Errors
    /// The default supplies no actual alias-prefix owner and refuses.
    fn command_alias_prefix_bytes_checked(
        &mut self,
        _cmd: CommandId,
    ) -> Result<Option<Vec<Vec<u8>>>, Self::OriginalNameError> {
        Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable("alias prefix owner").into())
    }

    /// Original report keys of actual live alias entries in this command table.
    ///
    /// # Errors
    /// The default supplies no actual alias inventory owner and refuses.
    fn aliases_in_bytes_checked(
        &self,
        _namespace: NsId,
    ) -> Result<Vec<Vec<u8>>, tcl_syntax::value::ValueError> {
        Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "alias inventory owner",
        ))
    }

    /// Variable lookup/listing policy of this execution engine, when selected.
    fn variable_lookup_policy(&self) -> Option<tcl_dialect::VariableLookupPolicy> {
        None
    }

    /// Resolve `name` (qualified or unqualified) from context `cxt` to the
    /// command it names, following the `cxt → namespace path → root` order. The
    /// returned handle is invoked via [`Commands::dispatch_id`].
    fn find_command(&self, cxt: NsId, name: &str) -> Option<CommandId>;
    /// The current namespace.
    fn current(&self) -> NsId;
    /// Actual root command lookup context, independently of namespace-tree
    /// lookup. Flat namespace engines must retain their genuine root object
    /// and live interpreter before supplying this context.
    ///
    /// # Errors
    /// Propagates unavailable or stale root naming authority. The default
    /// adapter uses the engine's checked C-style namespace lookup.
    fn root_command_context_checked(&self) -> Result<Option<NsId>, tcl_syntax::value::ValueError> {
        self.find_namespace_bytes_checked(self.current(), b"::")
    }

    /// Construct Jim's actual inventory List from retained current table keys.
    /// The owner selects original candidate positions with the shared naming
    /// policy and preserves their actual headers and command generations.
    ///
    /// # Errors
    /// The default supplies no native table/key/result constructor and refuses.
    fn native_jim_command_inventory_result(
        &mut self,
        _root: NsId,
        _pattern: Option<&[u8]>,
        _include_spaces: bool,
        _kind: NativeJimCommandInventoryKind,
    ) -> Result<<Self as tcl_syntax::value::ValueOps>::Value, tcl_syntax::value::ValueError>
    where
        Self: tcl_syntax::value::ValueOps,
    {
        Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "original Jim command inventory table keys",
        ))
    }
    /// The fully-qualified name of namespace `ns` (`"::"` for the global root) —
    /// what `namespace current` reports.
    fn name(&self, ns: NsId) -> String;
    /// The current fully qualified reporting name of the selected [`CommandId`],
    /// or `None` for a stale or unknown id. Backs `namespace which`.
    ///
    /// Distinct command slots can render as the same name. Retain the command
    /// handle for identity and dispatch independently of this reporting text.
    fn command_name(&self, cmd: CommandId) -> Option<String>;

    // -- namespace-tree navigation (mirrors C's `Namespace` struct: `nsId`
    // identity, `parentPtr`, `childTable`). A namespace *is* a handle; its
    // FQN/parent/children are queried from it. `name(ns)` (above) is its
    // `fullName`. These back `namespace exists`/`parent`/`children`.

    /// Resolve namespace `name` (qualified or unqualified) from context `cxt` to
    /// its table handle, or `None` if no such namespace table exists. During
    /// namespace teardown Tcl can retain that table for command enumeration
    /// after the public namespace token is dead; consumers requiring a public
    /// token must also consult [`Namespaces::namespace_is_live`].
    fn find_namespace(&self, cxt: NsId, name: &str) -> Option<NsId>;
    /// Whether a namespace table handle still denotes a public namespace token.
    /// Runtimes without a distinct teardown interval use the default.
    fn namespace_is_live(&self, ns: NsId) -> bool {
        let _ = ns;
        true
    }
    /// The handle of `ns`'s parent (`parentPtr`), or `None` for the global root.
    fn parent(&self, ns: NsId) -> Option<NsId>;
    /// The handles of `ns`'s direct child namespaces (`childTable`) in creation
    /// order.
    fn children(&self, ns: NsId) -> Vec<NsId>;
    /// The same live children in the observable `Tcl_FirstHashEntry` order of
    /// Tcl's retained string-key hash table. Adapters that model only a tree may
    /// use the creation-order default; `TclVM` adapters override this with the
    /// shared hash-table owner so resize and deletion history is preserved.
    fn children_hash_order(&self, ns: NsId) -> Vec<NsId> {
        self.children(ns)
    }

    // -- command enumeration (a namespace's `cmdTable`; backs `info commands`/
    // `info procs`). Direct members only — one level, not descendants — returned
    // as **unqualified** tail names. This is the command-listing **enumeration**
    // surface, the namespace analogue of `VarStore::array_keys`.

    /// The unqualified names of commands defined **directly** in `ns`. Mirrors a
    /// walk of `Namespace.cmdTable`; backs `info commands`.
    fn commands_in(&self, ns: NsId) -> Vec<String>;
    /// The unqualified names of **user procedures** defined directly in `ns` (the
    /// `TclIsProc` subset of [`commands_in`](Self::commands_in)); backs `info procs`.
    fn procs_in(&self, ns: NsId) -> Vec<String>;
    /// The unqualified names of **variables** defined directly in `ns` (a walk of
    /// `Namespace.varTable`); backs `info vars ::ns::*` and `info globals` (the
    /// global namespace's variables). The variable analogue of
    /// [`commands_in`](Self::commands_in).
    fn vars_in(&self, ns: NsId) -> Vec<String>;
    /// The direct `const` bindings in `ns`; link aliases are excluded.
    fn consts_in(&self, ns: NsId) -> Vec<String>;

    // -- resolution accessors the shared `namespace which -variable` / `origin`
    // cores need beyond navigation (`tcl_cmd_core::namespace`).

    /// Does namespace `ns`'s **own** variable table hold an entry named
    /// `simple` (an unqualified name)? This is `Tcl_FindNamespaceVar`'s single
    /// probe: the namespace's `varTable` only — never the call frame, so a
    /// proc local of the same name is invisible here. Backs
    /// [`which_variable`](../tcl_cmd_core/namespace/fn.which_variable.html).
    fn namespace_var_exists(&self, ns: NsId, simple: &str) -> bool;

    /// The command `cmd` was ultimately imported from — C's
    /// `TclGetOriginalCommand`, which is itself the whole walk (`while
    /// (cmdPtr->deleteProc == DeleteImportedCmd) cmdPtr = realCmdPtr`), not a
    /// single hop. `None` when `cmd` is not an imported command.
    ///
    /// The walk stays with the runtime because an import link is a command
    /// *token*, and a runtime whose tokens are name-keyed needs its own
    /// disambiguation (the VM's hidden/visible domains: `interp hide {} a b`
    /// leaves a hidden token `b` whose provenance must not be confused with an
    /// unrelated visible command also called `b`). Backs
    /// [`origin`](../tcl_cmd_core/namespace/fn.origin.html).
    fn command_origin(&self, cmd: CommandId) -> Option<CommandId>;

    // -- byte-valued spellings ------------------------------------------------
    //
    // A Tcl name is a byte string, not text: `set [binary format c 255] 1`
    // names a variable no `&str` can hold without a lossy round trip. The
    // Text adapters may implement the Unicode methods. Their byte defaults
    // perform checked projection and decline unrepresentable names; they never
    // manufacture a replacement spelling. Byte-native adapters override these
    // methods and preserve every stored row in byte enumeration and snapshots.

    /// [`find_command`](Self::find_command) over a byte-valued name.
    fn find_command_bytes(&self, cxt: NsId, name: &[u8]) -> Option<CommandId> {
        core::str::from_utf8(name)
            .ok()
            .and_then(|name| self.find_command(cxt, name))
    }

    /// Resolve an original native command operand while preserving host refusal
    /// separately from a proved missing binding.
    ///
    /// # Errors
    /// The adapter must issue a checked actual-context lookup; the default refuses.
    fn find_command_bytes_checked(
        &self,
        _cxt: NsId,
        _name: &[u8],
    ) -> Result<Option<CommandId>, tcl_syntax::value::ValueError> {
        Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "command lookup",
        ))
    }

    /// Resolve a native namespace operand without converting unsupported
    /// protocol/context into a negative namespace result.
    ///
    /// # Errors
    /// The adapter must issue a checked actual-context lookup; the default refuses.
    fn find_namespace_bytes_checked(
        &self,
        _cxt: NsId,
        _name: &[u8],
    ) -> Result<Option<NsId>, tcl_syntax::value::ValueError> {
        Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "namespace lookup",
        ))
    }

    /// Probe an actual parent's child table by its original physical member key.
    /// This operation does not parse a written namespace name or consult a cache.
    ///
    /// # Errors
    /// Refuses when the backend cannot authenticate the selected child table.
    fn find_namespace_child_bytes_checked(
        &self,
        _parent: NsId,
        _member: &[u8],
    ) -> Result<Option<NsId>, tcl_syntax::value::ValueError> {
        Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "namespace child-table lookup",
        ))
    }

    /// Resolve the native namespace-variable query operation and report the
    /// followed namespace cell name. This is distinct from ordinary scalar
    /// access and must preserve the query operation's operand extent.
    ///
    /// # Errors
    /// Refuses when the adapter cannot authenticate the actual query protocol.
    fn namespace_variable_name_bytes_checked(
        &self,
        _cxt: NsId,
        _name: &[u8],
    ) -> Result<Option<Vec<u8>>, tcl_syntax::value::ValueError> {
        Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "namespace variable query",
        ))
    }

    /// [`find_namespace`](Self::find_namespace) over a byte-valued name.
    fn find_namespace_bytes(&self, cxt: NsId, name: &[u8]) -> Option<NsId> {
        core::str::from_utf8(name)
            .ok()
            .and_then(|name| self.find_namespace(cxt, name))
    }

    /// [`namespace_var_exists`](Self::namespace_var_exists) over a
    /// byte-valued simple name.
    fn namespace_var_exists_bytes(&self, ns: NsId, simple: &[u8]) -> bool {
        core::str::from_utf8(simple).is_ok_and(|simple| self.namespace_var_exists(ns, simple))
    }

    /// Snapshot selected namespace report data from its actual physical owner.
    /// This is separate from Unicode/report spelling and supplies no live handle.
    /// Implementations preserve the selected lifecycle and original getter stages.
    ///
    /// # Errors
    /// The default has no original namespace report owner and refuses explicitly.
    fn namespace_report_parts_checked(
        &mut self,
        _namespace: NsId,
    ) -> Result<NativeNamespaceReportParts, Self::OriginalNameError> {
        Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "original namespace report parts",
        )
        .into())
    }

    /// [`name`](Self::name) as the bytes the namespace is actually keyed by.
    fn name_bytes(&self, ns: NsId) -> Vec<u8> {
        self.name(ns).into_bytes()
    }

    /// [`command_name`](Self::command_name) as report bytes. The default encodes
    /// its String result as UTF-8; engines can preserve exact report bytes.
    /// Rendered bytes carry no command-table identity.
    fn command_name_bytes(&self, cmd: CommandId) -> Option<Vec<u8>> {
        self.command_name(cmd).map(String::into_bytes)
    }

    /// Direct command-table members without Unicode decoding.
    fn commands_in_bytes(&self, ns: NsId) -> Vec<Vec<u8>> {
        self.commands_in(ns)
            .into_iter()
            .map(String::into_bytes)
            .collect()
    }

    /// Direct procedure-table members without Unicode decoding.
    fn procs_in_bytes(&self, ns: NsId) -> Vec<Vec<u8>> {
        self.procs_in(ns)
            .into_iter()
            .map(String::into_bytes)
            .collect()
    }

    /// [`vars_in`](Self::vars_in) without a lossy UTF-8 round trip.
    fn vars_in_bytes(&self, ns: NsId) -> Vec<Vec<u8>> {
        self.vars_in(ns)
            .into_iter()
            .map(String::into_bytes)
            .collect()
    }

    /// Ordered owned namespace inventory. Native adapters validate actual
    /// table selection; this compatibility default does not issue a hash recipe.
    fn vars_in_bytes_checked(
        &self,
        ns: NsId,
    ) -> Result<Vec<Vec<u8>>, tcl_syntax::value::ValueError> {
        Ok(self.vars_in_bytes(ns))
    }

    /// [`consts_in`](Self::consts_in) without a lossy UTF-8 round trip.
    fn consts_in_bytes(&self, ns: NsId) -> Vec<Vec<u8>> {
        self.consts_in(ns)
            .into_iter()
            .map(String::into_bytes)
            .collect()
    }
}
