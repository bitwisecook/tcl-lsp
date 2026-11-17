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

//! Reference bodies on the VM's own compile of a procedure body: the caller is
//! lowered under the runtime's live native entry, where every command needs
//! the native selection the runtime replays, so a spliced body is lowered under
//! that same entry and moved into the caller's source with its selections.
//!
//! What a definition splices in is still decided where [`reference::import`]
//! decides it, in the definition read by a fresh interpreter of the release.
//! The live lowering of the body only supplies the statements, which must stand
//! where that reading's statements stand; a body whose two readings differ, or
//! whose selections reach a source the caller's does not hold, stays a call.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use tcl_compiler::analyser::ResolvedAnalysisInput;
use tcl_compiler::command_binding::{SourceAnalysisOptions, SourceNamespaceKey, SourceOriginId};
use tcl_compiler::expr_ast::ExprNode;
use tcl_compiler::ir::{Module, Procedure, ProcedureId, ReferenceImport, Script, Statement};
use tcl_lexer::{LexerConfig, SourceImage, Span};
use tcl_registry::CommandRegistry;

use super::InlineSpec;
use super::reference::{self, Imported, SiteKey};

/// The runtime entry the caller was lowered under, and the fresh reading of
/// the caller's release that decides what a definition splices in.
#[derive(Clone, Copy)]
pub(super) struct LiveEntry<'a> {
    pub(super) options: SourceAnalysisOptions<'a>,
    pub(super) fresh: &'a ResolvedAnalysisInput,
}

/// What lowers a body under the live entry: the caller's compile.
struct Lowering<'a> {
    registry: &'a CommandRegistry,
    config: LexerConfig,
    profile: &'static tcl_dialect::DialectProfile,
    options: SourceAnalysisOptions<'a>,
}

impl Lowering<'_> {
    /// `body` lowered under the live entry as the body of a procedure with
    /// `parameters` made in `namespace` (rooted), offsets relative to the body.
    fn body(&self, body: &str, parameters: &[String], namespace: &str) -> Script {
        // The definition's own formals, not the caller's.
        let mut options = self.options;
        options.incoming_formals = &[];
        tcl_compiler::lowering::lower_procedure_target_module_for_bytecode_with_options(
            tcl_runtime_api::ProcedureCompileTarget {
                source: body,
                parameters,
                namespace: tcl_syntax::naming::unroot_rooted_key(namespace).unwrap_or(namespace),
            },
            self.registry,
            self.config,
            Some(self.profile),
            false,
            Some(options),
        )
        .top_level
    }
}

/// A parameterised definition, kept so each call site can lower its body
/// again with that site's renames written into the text.
struct RenamedDefinition {
    body: String,
    parameters: Vec<String>,
    namespace: String,
    namespace_context: Option<Box<SourceNamespaceKey>>,
    /// The body lowered under the live entry, offsets relative to the body.
    lowered: Script,
}

/// The definitions a live compile imported, and the source their spliced
/// bodies are moved into, which grows by a renamed body at each site.
pub(super) struct LiveBodies<'a> {
    lowering: Lowering<'a>,
    renamed: HashMap<ProcedureId, RenamedDefinition>,
    image: RefCell<SourceImage>,
}

impl LiveBodies<'_> {
    /// The source the module ends with.
    pub(super) fn into_image(self) -> SourceImage {
        self.image.into_inner()
    }

    /// The body of `id` with `names` applied, lowered under the live entry
    /// from its renamed text and moved into the module's source: a replay of
    /// any of its commands reads the renamed variables. `None` when the text
    /// cannot be renamed or its lowering is not the IR rename of the body.
    pub(super) fn renamed(
        &self,
        id: ProcedureId,
        names: &HashMap<String, String>,
    ) -> Option<Script> {
        let definition = self.renamed.get(&id)?;
        let config = self.lowering.config;
        let text =
            super::live_rename::renamed_text(&definition.body, &definition.lowered, names, config)?;
        let parameters: Vec<String> = definition
            .parameters
            .iter()
            .map(|name| names.get(name).cloned())
            .collect::<Option<_>>()?;
        let mut lowered = self
            .lowering
            .body(&text, &parameters, &definition.namespace);
        let expected = super::rename::rewrite_script(&definition.lowered, names, config);
        if view(&expected) != view(&lowered) {
            return None;
        }
        let mut image = self.image.borrow_mut();
        let start = u32::try_from(image.len() + 1).ok()?;
        start.checked_add(u32::try_from(text.len()).ok()?)?;
        let mut bytes = image.bytes().to_vec();
        bytes.push(b'\n');
        bytes.extend_from_slice(text.as_bytes());
        let grown = SourceImage::from_bytes(bytes, image.channel());
        if !moved(&mut lowered, &text, start, &grown) {
            return None;
        }
        *image = grown;
        // As the IR rename does: the renamed expression's offsets are not the
        // preparation's, and the command it was folded from stays bound
        // through its binding site.
        tcl_compiler::ir::for_each_script_mut(&mut lowered, &mut |script| {
            for statement in &mut script.statements {
                if let Statement::Return { expr_base, .. } = statement {
                    *expr_base = None;
                }
            }
        });
        lowered
            .namespace_context
            .clone_from(&definition.namespace_context);
        Some(lowered)
    }
}

/// [`reference::import`] for a module lowered under a live native entry.
pub(super) fn import<'a>(
    module: &mut Module,
    registry: &'a CommandRegistry,
    config: LexerConfig,
    profile: &'static tcl_dialect::DialectProfile,
    entry: LiveEntry<'a>,
    called: &HashMap<String, Vec<SiteKey>>,
) -> (Imported, LiveBodies<'a>) {
    let mut imported = Imported::default();
    let lowering = Lowering {
        registry,
        config,
        profile,
        options: entry.options,
    };
    let mut renamed = HashMap::new();
    for (spec, text) in registry.reference_bodies() {
        let Some(sites) = called.get(spec.name) else {
            continue;
        };
        let Some(facts) = tcl_compiler::site_claims::reference_body_facts(registry, spec) else {
            continue;
        };
        let Some((definition, Some(inlinable))) = reference::lower_definition(
            text,
            spec,
            registry,
            config,
            Some(profile),
            Some(entry.fresh),
        ) else {
            continue;
        };
        let qualified = definition.qualified_name.clone();
        if module.procedures.has_display(&qualified)
            || module.redefined_procedures.contains(&qualified)
        {
            continue;
        }
        let Some((proc, lowered)) = place(module, text, definition, &lowering) else {
            continue;
        };
        module.reference_bodies.imports.push(ReferenceImport {
            name: qualified,
            parameters: proc.params_raw.clone(),
            body: proc.body_source.clone().unwrap_or_default(),
            backing: spec.runtime_backing.kind(),
            facts,
        });
        let inlinable = inlinable.moved_to(&proc);
        let parameterised =
            matches!(inlinable, InlineSpec::Parameterised(..)).then(|| RenamedDefinition {
                body: proc.body_source.clone().unwrap_or_default(),
                parameters: proc.params.clone(),
                namespace: namespace_of(&proc).unwrap_or_default(),
                namespace_context: proc.body.namespace_context.clone(),
                lowered,
            });
        let id = module.retain_procedure(proc);
        if let Some(definition) = parameterised {
            renamed.insert(id, definition);
        }
        imported.ids.insert(id);
        imported.inlinable.insert(id, inlinable);
        imported.sites.extend(sites.iter().map(|site| (*site, id)));
    }
    let bodies = LiveBodies {
        lowering,
        renamed,
        image: RefCell::new(module.source.clone()),
    };
    (imported, bodies)
}

/// The calls of `module` whose execution the live entry proves reaches one
/// command a pack defines with a body, by that command's name; `None` for none.
/// The splice still holds each site to the definition through the procedure
/// binding it records, which the runtime checks before running it.
pub(super) fn called_commands(
    module: &Module,
    registry: &CommandRegistry,
) -> Option<HashMap<String, Vec<SiteKey>>> {
    let mut called: HashMap<String, Vec<SiteKey>> = HashMap::new();
    let mut note = |statement: &Statement| {
        if !matches!(statement, Statement::Call { .. }) {
            return;
        }
        let Some(tokens) = statement.tokens() else {
            return;
        };
        let Some(target) = tokens.source_binding.as_ref().and_then(
            tcl_compiler::command_binding::SourceInvocationBinding::proved_execution_target,
        ) else {
            return;
        };
        if !target.prepended.is_empty() {
            return;
        }
        let name = tcl_syntax::naming::unroot_rooted_key(&target.command)
            .unwrap_or(&target.command)
            .to_owned();
        if let Some(site) = reference::site_key(tokens) {
            called.entry(name).or_default().push(site);
        }
    };
    tcl_compiler::ir::for_each_statement(&module.top_level, &mut note);
    for proc in module.procedures.values() {
        tcl_compiler::ir::for_each_statement(&proc.body, &mut note);
    }
    called.retain(|name, _| {
        registry
            .reference_bodies()
            .any(|(spec, _)| spec.name == name.as_str())
    });
    (!called.is_empty()).then_some(called)
}

/// The rooted namespace a definition was made in.
fn namespace_of(definition: &Procedure) -> Option<String> {
    definition.body.namespace_context.as_ref()?.advisory_key()
}

/// Lower `definition`'s body under the live entry, check it against the fresh
/// reading, and append the definition to the module's source with the live body
/// and its selections moved there. Also returns the live body as lowered.
fn place(
    module: &mut Module,
    text: &str,
    mut definition: Procedure,
    lowering: &Lowering<'_>,
) -> Option<(Procedure, Script)> {
    let body = definition.body_source.clone()?;
    let lowered = lowering.body(&body, &definition.params, &namespace_of(&definition)?);
    let mut fresh = definition.body.clone();
    tcl_compiler::lattice_rebase::rebase_script(&mut fresh, -i64::from(definition.body_offset));
    if shape(&fresh) != shape(&lowered) {
        return None;
    }
    let (start, image) = reference::appended(module, text)?;
    let mut live = lowered.clone();
    if !moved(
        &mut live,
        &body,
        start.checked_add(definition.body_offset)?,
        &image,
    ) {
        return None;
    }
    reference::shift_definition(&mut definition, start);
    // The namespace the definition was made in, which the live lowering of a
    // bare body does not state.
    live.namespace_context = definition.body.namespace_context.take();
    definition.body = live;
    reference::commit(module, image);
    Some((definition, lowered))
}

/// Move `script`, lowered from `body`, to where `body` stands at `delta` in
/// `image`, with the native selections of its commands.
fn moved(script: &mut Script, body: &str, delta: u32, image: &SourceImage) -> bool {
    let Some(from) = body_origin(script, body) else {
        tcl_compiler::lattice_rebase::rebase_script(script, i64::from(delta));
        return true;
    };
    let onto = Arc::new(SourceOriginId::authored_image(image.clone()));
    tcl_compiler::command_binding::relocate_script(script, delta, &from, &onto)
}

/// Each statement of `script`, nested ones included, by kind and span.
fn shape(script: &Script) -> Vec<(std::mem::Discriminant<Statement>, Span)> {
    let mut shape = Vec::new();
    tcl_compiler::ir::for_each_statement(script, &mut |statement| {
        shape.push((std::mem::discriminant(statement), statement.span()));
    });
    shape
}

/// Each statement of `script`, nested ones included, by kind and the names and
/// values the splice's rename rewrites, without their offsets.
fn view(script: &Script) -> Vec<(std::mem::Discriminant<Statement>, Vec<String>)> {
    let mut view = Vec::new();
    tcl_compiler::ir::for_each_statement(script, &mut |statement| {
        let mut fields = Vec::new();
        match statement {
            Statement::AssignConst { name, value, .. }
            | Statement::AssignValue { name, value, .. } => {
                fields.extend([name.clone(), value.clone()]);
            }
            Statement::AssignExpr { name, expr, .. } => {
                fields.push(name.clone());
                expression(expr, &mut fields);
            }
            Statement::Incr { name, amount, .. } => {
                fields.push(name.clone());
                fields.extend(amount.iter().cloned());
            }
            Statement::ExprEval { expr, .. } => expression(expr, &mut fields),
            Statement::Call {
                command,
                args,
                defs,
                reads,
                ..
            } => {
                fields.push(command.clone());
                fields.extend(args.iter().chain(defs).chain(reads).cloned());
            }
            Statement::Return { value, expr, .. } => {
                fields.extend(value.iter().cloned());
                if let Some(expr) = expr {
                    expression(expr, &mut fields);
                }
            }
            Statement::If { clauses, .. } => {
                for clause in clauses {
                    expression(&clause.condition, &mut fields);
                }
            }
            Statement::While { condition, .. } | Statement::For { condition, .. } => {
                expression(condition, &mut fields);
            }
            Statement::Foreach { iterators, .. } => {
                for iterator in iterators {
                    fields.extend(iterator.vars.iter().cloned());
                    fields.push(iterator.list_arg.clone());
                }
            }
            Statement::Catch {
                result_var,
                options_var,
                ..
            } => fields.extend(result_var.iter().chain(options_var).cloned()),
            Statement::Switch { subject, .. } => fields.push(subject.clone()),
            _ => {}
        }
        view.push((std::mem::discriminant(statement), fields));
    });
    view
}

/// The variables and operators of an expression, in order, without offsets.
fn expression(node: &ExprNode, fields: &mut Vec<String>) {
    let mut pending = vec![node];
    while let Some(node) = pending.pop() {
        match node {
            ExprNode::Var { text, name, .. } => fields.extend([text.clone(), name.clone()]),
            ExprNode::Binary { op, left, right } => {
                fields.push(format!("{op:?}"));
                pending.extend([&**right, &**left]);
            }
            ExprNode::Unary { op, operand } => {
                fields.push(format!("{op:?}"));
                pending.push(operand);
            }
            ExprNode::Ternary {
                condition,
                true_branch,
                false_branch,
            } => pending.extend([&**false_branch, &**true_branch, &**condition]),
            ExprNode::Call { function, args, .. } => {
                fields.push(function.clone());
                pending.extend(args.iter().rev());
            }
            ExprNode::CompiledWord { text, .. } => fields.push(text.clone()),
            ExprNode::Literal { .. }
            | ExprNode::String { .. }
            | ExprNode::Command { .. }
            | ExprNode::Raw { .. } => fields.push(String::from("·")),
        }
    }
}

/// The source instance the live lowering allocated the body's commands in.
fn body_origin(script: &mut Script, body: &str) -> Option<Arc<SourceOriginId>> {
    let mut origin = None;
    tcl_compiler::command_binding::for_each_script_allocation_site_mut(script, &mut |site| {
        if origin.is_none() && site.source.source_image().bytes() == body.as_bytes() {
            origin = Some(Arc::clone(&site.source));
        }
    });
    origin
}
