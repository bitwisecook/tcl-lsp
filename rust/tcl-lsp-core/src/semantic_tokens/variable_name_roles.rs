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

//! Which argument words of a procedure call name a variable or a command.

use rustc_hash::{FxHashMap, FxHashSet};
use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::analyser::types::{ProcArgTrait, ProcDef};

/// Extra "this argument names a written variable" positions for commands the
/// static [`CommandRegistry`] does not model — user procs whose parameters the
/// analyser inferred to alias a caller variable (`upvar $param`), and
/// `# tcl-lsp: stub … :var` / `:var_read` declarations.  Keyed by command /
/// proc name; each value is the 0-based argument indices (head excluded) that
/// name a variable — split by direction, since a *written* target highlights as
/// a `Variable` declaration and a *read* reference as a plain `Variable`.  Lets
/// the retag highlight `myset arr(key) …` / `myexists arr(key)` the same way it
/// highlights `set arr(key) …` / `info exists arr(key)`.
///
/// Built from an [`AnalysisResult`] (single file) or merged across a project's
/// files.  Empty (and cost-free) on the pure-segmentation path, where only the
/// static registry roles apply.  Stub roles are source-derived and unioned in
/// at token-collection time, so they apply on every path without threading.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VarNameArgRoles {
    pub(super) write: FxHashMap<String, Vec<u32>>,
    pub(super) read: FxHashMap<String, Vec<u32>>,
    /// Argument positions whose value the analyser inferred to name a *command*
    /// ([`ProcArgTrait::Command`] — a `$param` command head or a `CommandPrefix`
    /// argument), so a literal call-site arg highlights as a command.
    pub(super) command: FxHashMap<String, Vec<u32>>,
    /// Names this index *abstained* on because the procs it was built from
    /// disagreed about their indices, per direction.
    ///
    /// Carried rather than discarded so [`VarNameArgRoles::merge`] can fold
    /// per-file indexes into a project-wide one that equals
    /// [`VarNameArgRoles::from_procs`] over every file's procs at once: without
    /// it, a name one file already dropped as ambiguous would silently be
    /// re-adopted from another file's unambiguous entry.
    pub(super) ambiguous: RoleAmbiguity,
    pub(super) original: Vec<OriginalProcArgRoles>,
    pub(super) project_source:
        Vec<tcl_compiler::registry_invocation::source_structure::OriginalProjectProcedureRoles>,
}

/// Retained byte publication and declaration identity for inferred assistance.
/// These roles establish no actual variable binding or compiler preparation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct OriginalProcArgRoles {
    pub(super) name: tcl_compiler::signature_scan::scope::SignatureSourceCommand,
    pub(super) site: Option<tcl_names::command::CommandAllocationSite>,
    pub(super) write: Vec<u32>,
    pub(super) read: Vec<u32>,
    pub(super) command: Vec<u32>,
}

impl OriginalProcArgRoles {
    pub(super) fn new(
        name: &tcl_compiler::signature_scan::scope::SignatureSourceCommand,
        site: Option<&tcl_names::command::CommandAllocationSite>,
        proc_def: &ProcDef,
    ) -> Self {
        Self {
            name: name.clone(),
            site: site.cloned(),
            write: proc_var_write_indices(proc_def),
            read: proc_var_read_indices(proc_def),
            command: proc_command_indices(proc_def),
        }
    }

    fn same_roles(&self, other: &Self) -> bool {
        self.write == other.write && self.read == other.read && self.command == other.command
    }
}

/// The per-direction abstention sets of a [`VarNameArgRoles`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct RoleAmbiguity {
    pub(super) write: FxHashSet<String>,
    pub(super) read: FxHashSet<String>,
    pub(super) command: FxHashSet<String>,
}

impl VarNameArgRoles {
    /// Retain genuine declaring source/formal owners for conditional cross-file
    /// display. Report maps never select a call or reconstruct a formal binding.
    #[must_use]
    pub fn from_project_source_roles(
        providers: Vec<
            tcl_compiler::registry_invocation::source_structure::OriginalProjectProcedureRoles,
        >,
    ) -> Self {
        Self {
            project_source: providers,
            ..Self::default()
        }
    }

    /// Exact original cross-document provider transport, independently of the
    /// compatibility maps used by standalone callers.
    #[must_use]
    pub fn project_source_roles(
        &self,
    ) -> &[tcl_compiler::registry_invocation::source_structure::OriginalProjectProcedureRoles] {
        &self.project_source
    }

    /// Infer the variable-name argument positions of every proc in `analysis`.
    #[must_use]
    pub fn from_analysis(analysis: &AnalysisResult) -> Self {
        let mut result =
            Self::from_original_declarations(analysis.original_procedure_declarations());
        if analysis.allows_lexical_declaration_advice() {
            let lexical = Self::from_procs(
                analysis
                    .all_procs
                    .values()
                    .filter(|proc_def| proc_def.source_name.is_none()),
            );
            result = Self::merge([&result, &lexical]);
        }
        result
    }

    /// Preserve original declarations even when their UI names collide.
    #[must_use]
    pub fn from_original_declarations<'a>(
        declarations: impl IntoIterator<
            Item = &'a tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
                ProcDef,
            >,
        >,
    ) -> Self {
        Self {
            original: declarations
                .into_iter()
                .map(|record| {
                    OriginalProcArgRoles::new(
                        record.name(),
                        Some(record.declaration_site()),
                        record.metadata(),
                    )
                })
                .collect(),
            ..Self::default()
        }
    }

    /// Infer from an iterator of proc definitions — a single file's, or a whole
    /// project's files chained together.  A proc name that resolves to two
    /// *different non-empty* index sets across the iterator is dropped as
    /// ambiguous, so the result is independent of iteration order — matching the
    /// highlight-only, sound-by-abstention posture of the cross-file class
    /// index.  An empty index set (the proc has no by-reference argument in that
    /// direction) never participates: it neither seeds nor conflicts an entry,
    /// so a definition that contributes roles is not cancelled by one that
    /// contributes none.
    #[must_use]
    pub fn from_procs<'a>(procs: impl IntoIterator<Item = &'a ProcDef>) -> Self {
        let mut write = RoleMapBuilder::default();
        let mut read = RoleMapBuilder::default();
        let mut command = RoleMapBuilder::default();
        let mut original = Vec::new();
        for proc in procs {
            if let Some(name) = &proc.source_name {
                original.push(OriginalProcArgRoles::new(name, None, proc));
                continue;
            }
            let keys = proc_name_keys(proc);
            write.insert(&keys, &proc_var_write_indices(proc));
            read.insert(&keys, &proc_var_read_indices(proc));
            command.insert(&keys, &proc_command_indices(proc));
        }
        let mut result = Self::from_builders(write, read, command);
        result.original = original;
        result
    }

    /// Fold per-file indexes into one project-wide index, applying the same
    /// abstain-on-conflict rule [`Self::from_procs`] applies within a file — and
    /// inheriting each part's own abstentions, so the result is exactly
    /// `from_procs` over every part's procs concatenated, and independent of the
    /// order the parts arrive in.
    #[must_use]
    pub fn merge<'a>(parts: impl IntoIterator<Item = &'a Self>) -> Self {
        let mut write = RoleMapBuilder::default();
        let mut read = RoleMapBuilder::default();
        let mut command = RoleMapBuilder::default();
        let mut original = Vec::new();
        let mut project_source = Vec::new();
        for part in parts {
            project_source.extend(part.project_source.iter().cloned());
            original.extend(part.original.iter().cloned());
            write.absorb(&part.write, &part.ambiguous.write);
            read.absorb(&part.read, &part.ambiguous.read);
            command.absorb(&part.command, &part.ambiguous.command);
        }
        let mut result = Self::from_builders(write, read, command);
        result.original = original;
        result.project_source = project_source;
        result
    }

    /// Close the three per-direction builders into an index, keeping each
    /// direction's abstentions alongside its map.
    fn from_builders(write: RoleMapBuilder, read: RoleMapBuilder, command: RoleMapBuilder) -> Self {
        let (write, write_ambiguous) = write.finish();
        let (read, read_ambiguous) = read.finish();
        let (command, command_ambiguous) = command.finish();
        Self {
            write,
            read,
            command,
            original: Vec::new(),
            project_source: Vec::new(),
            ambiguous: RoleAmbiguity {
                write: write_ambiguous,
                read: read_ambiguous,
                command: command_ambiguous,
            },
        }
    }

    /// `true` when no command carries an inferred by-reference argument.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.write.is_empty()
            && self.read.is_empty()
            && self.command.is_empty()
            && self
                .project_source
                .iter()
                .all(|provider| !provider.has_roles())
            && self.original.iter().all(|roles| {
                roles.write.is_empty() && roles.read.is_empty() && roles.command.is_empty()
            })
    }

    pub(super) fn original_roles_at(
        &self,
        analysis: &AnalysisResult,
        source: &str,
        offset: u32,
    ) -> Option<OriginalProcArgRoles> {
        let image = tcl_lexer::SourceImage::document(source);
        let config = analysis.body_lexer_config?;
        if !analysis.matches_original_source_image(&image, config) {
            return None;
        }
        let invocation = crate::definition::invocation_reference_at(analysis, offset)?;
        if invocation.range.start() != offset {
            return None;
        }
        let input = invocation.original_name_input.as_ref()?;
        let lookup = invocation
            .original_lookup
            .as_ref()
            .filter(|lookup| lookup.name_input() == input)?;
        if lookup.site().offset != offset || lookup.site().source.source_image() != &image {
            return None;
        }
        if let Some(reference) = &invocation.resolved_command_reference {
            use tcl_compiler::command_binding::SourceCommandReferenceBinding;
            use tcl_names::command::BindingKind;
            let kind = match reference.binding() {
                SourceCommandReferenceBinding::Direct { kind, .. }
                | SourceCommandReferenceBinding::Imported { kind, .. } => kind,
            };
            // Alias prefixes need an independent effective argument mapping.
            if *kind != BindingKind::Proc {
                return None;
            }
            let definition = reference
                .linked_definition()
                .or_else(|| reference.definition())?;
            if let Some(proc_def) = analysis.proc_for_definition(definition, source) {
                let name = proc_def.source_name.as_ref()?;
                return Some(OriginalProcArgRoles::new(
                    name,
                    Some(&definition.allocation().site),
                    proc_def,
                ));
            }
            let mut selected = self
                .original
                .iter()
                .filter(|roles| roles.site.as_ref() == Some(&definition.allocation().site));
            let first = selected.next()?;
            return selected
                .all(|other| first.same_roles(other))
                .then(|| first.clone());
        }
        // Declaration assistance only: all original lookup alternatives and
        // all records at the selected byte publication must agree on roles.
        let selected =
            lookup.matching_publications(self.original.iter().map(|roles| (&roles.name, roles)))?;
        let first = *selected.first()?;
        selected
            .iter()
            .all(|other| first.same_roles(other))
            .then(|| first.clone())
    }
}

/// Accumulates one direction's `name → arg indices` map while dropping any name
/// that resolves to conflicting index sets (abstain-on-conflict), so the merged
/// result is independent of insertion order.
#[derive(Default)]
struct RoleMapBuilder {
    map: FxHashMap<String, Vec<u32>>,
    pub(super) ambiguous: FxHashSet<String>,
}

impl RoleMapBuilder {
    pub(super) fn insert(&mut self, keys: &[String], indices: &[u32]) {
        if indices.is_empty() {
            return;
        }
        for key in keys {
            if self.ambiguous.contains(key) {
                continue;
            }
            match self.map.get(key) {
                Some(existing) if existing.as_slice() != indices => {
                    self.map.remove(key);
                    self.ambiguous.insert(key.clone());
                }
                Some(_) => {}
                None => {
                    self.map.insert(key.clone(), indices.to_vec());
                }
            }
        }
    }

    /// Fold an already-built map and its abstention set in, so merging finished
    /// indexes reaches the same fixed point as inserting every underlying proc.
    fn absorb(&mut self, map: &FxHashMap<String, Vec<u32>>, ambiguous: &FxHashSet<String>) {
        for key in ambiguous {
            self.map.remove(key);
            self.ambiguous.insert(key.clone());
        }
        for (key, indices) in map {
            self.insert(std::slice::from_ref(key), indices);
        }
    }

    fn finish(self) -> (FxHashMap<String, Vec<u32>>, FxHashSet<String>) {
        (self.map, self.ambiguous)
    }
}

/// The 0-based argument indices of a proc's parameters that the analyser
/// inferred to *alias a caller variable written by the proc*
/// ([`ProcArgTrait::VarWrite`]) — so a literal name passed there names the
/// caller's variable.  [`ProcArgTrait::DynamicNameLocal`] (the param's *value*
/// names a callee-local variable) is excluded: a literal there is not the
/// caller's variable.
pub(super) fn proc_var_write_indices(proc: &ProcDef) -> Vec<u32> {
    proc_indices_with_trait(proc, |traits| traits.contains(&ProcArgTrait::VarWrite))
}

/// The 0-based argument indices of a proc's parameters whose value the analyser
/// inferred to name a variable that is *read* — either a caller-frame
/// [`ProcArgTrait::VarRead`] `upvar` alias, or a
/// [`ProcArgTrait::DynamicNameLocal`] whose value names a callee-local variable
/// read (`set $p`, `set ${v}($k)`).  Both mean the literal at the call site is
/// a variable name, so both elevate to a read reference — the highlighter does
/// not need to distinguish caller-frame from callee-local.
pub(super) fn proc_var_read_indices(proc: &ProcDef) -> Vec<u32> {
    proc_indices_with_trait(proc, |traits| {
        traits.contains(&ProcArgTrait::VarRead) || traits.contains(&ProcArgTrait::DynamicNameLocal)
    })
}

/// The 0-based argument indices of a proc's parameters whose value the analyser
/// inferred to name a *command* ([`ProcArgTrait::Command`] — a `$param` command
/// head or a `CommandPrefix` argument), so a literal at the call site highlights
/// as a command.
pub(super) fn proc_command_indices(proc: &ProcDef) -> Vec<u32> {
    proc_indices_with_trait(proc, |traits| traits.contains(&ProcArgTrait::Command))
}

/// Shared body of [`proc_var_write_indices`] / [`proc_var_read_indices`]: the
/// positions of the parameters whose inferred trait set satisfies `keep`.
fn proc_indices_with_trait(
    proc: &ProcDef,
    keep: impl Fn(&std::collections::HashSet<ProcArgTrait>) -> bool,
) -> Vec<u32> {
    proc.params
        .iter()
        .enumerate()
        .filter(|(_, p)| proc.param_traits.get(&p.name).is_some_and(&keep))
        .filter_map(|(i, _)| u32::try_from(i).ok())
        .collect()
}

/// Every name a call site might use for `proc`: the bare name, the qualified
/// name, and the qualified name without its leading `::`.
fn proc_name_keys(proc: &ProcDef) -> Vec<String> {
    let mut keys = vec![proc.name.clone()];
    let stripped =
        tcl_syntax::naming::unroot_rooted_key(&proc.qualified_name).unwrap_or(&proc.qualified_name);
    if stripped != proc.name {
        keys.push(stripped.to_owned());
    }
    if proc.qualified_name != proc.name && proc.qualified_name != stripped {
        keys.push(proc.qualified_name.clone());
    }
    keys
}
