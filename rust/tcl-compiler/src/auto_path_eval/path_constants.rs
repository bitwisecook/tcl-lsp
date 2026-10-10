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

//! Authored path-variable inventory with retained naming and lexical homes.

use std::collections::{HashMap, HashSet};
use tcl_dialect::DialectProfile;
use tcl_syntax::naming::{NamePolicyProtocol, NativeNameContext, NativeNameProtocol};

use super::{assigned_name_value_indices, is_plain_scalar_name, namespace_body_index, raw_value};

mod original;
pub(crate) use original::extend_path_constant_assignments_from_analysis_commands;
pub use original::{constant_path_assignments_from_analysis, path_source_word_is_available};

/// A selected global home or an original lexical local activation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Home {
    Global,
    Local(u32),
}

/// An original accepted namespace-body interval, not an executed frame receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Scope {
    id: u32,
    start: u32,
    end: u32,
    namespace: String,
    local: bool,
}

/// One path write, declaration, or unavailable value in its selected home.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathConstantWrite {
    /// Selected variable key; Jim keys retain their original colon runs.
    pub name: String,
    /// Constructed namespace of the original lexical command.
    pub ns: String,
    /// Original command byte offset.
    pub at: u32,
    /// Recorded value or mutation.
    pub value: PathConstantValue,
    home: Home,
    scope: u32,
    alias: Option<String>,
}

/// The value contribution of a path-variable observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathConstantValue {
    /// Original value word and its substitution suppression.
    Raw {
        /// Word contents.
        text: String,
        /// Braced single-token value.
        literal: bool,
    },
    /// A checked original source expression with typed selected operations
    /// and authentic effective argument values. This supplies no native value.
    Original(super::OriginalSourcePathExpression),
    /// Membership and optional local link without an assignment.
    Declared,
    /// Mutation with an unavailable value or receiver.
    Poisoned,
}

/// Raw path observations retaining a naming issuer even when there are no rows.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PathConstantAssignments {
    policy: Option<NamePolicyProtocol>,
    source_config: Option<tcl_lexer::LexerConfig>,
    source_input: Option<crate::analyser::ResolvedAnalysisInput>,
    source_image: Option<tcl_lexer::SourceImage>,
    source_realm: Option<std::sync::Arc<crate::realm::CommandBindingRealm>>,
    recorded_end: u32,
    writes: Vec<PathConstantWrite>,
    scopes: Vec<Scope>,
    barriers: Vec<u32>,
    unavailable_scopes: Vec<(u32, u32)>,
}
impl PathConstantAssignments {
    /// Shared empty unknown inventory for absent documents.
    #[must_use]
    pub fn unknown() -> &'static Self {
        static UNKNOWN: PathConstantAssignments = PathConstantAssignments {
            policy: None,
            source_config: None,
            source_input: None,
            source_image: None,
            source_realm: None,
            recorded_end: 0,
            writes: Vec::new(),
            scopes: Vec::new(),
            barriers: Vec::new(),
            unavailable_scopes: Vec::new(),
        };
        &UNKNOWN
    }
    /// The independently selected authored or native naming provider.
    #[must_use]
    pub const fn naming_policy(&self) -> Option<NamePolicyProtocol> {
        self.policy
    }
    /// Exact immutable source inventory owner, independent of folded values.
    pub(super) fn matches_source_owner(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        image: &tcl_lexer::SourceImage,
        realm: &std::sync::Arc<crate::realm::CommandBindingRealm>,
    ) -> bool {
        self.source_input.as_ref() == Some(input)
            && self.source_config == Some(input.lexer_config())
            && self.source_image.as_ref() == Some(image)
            && self
                .source_realm
                .as_ref()
                .is_some_and(|original| std::sync::Arc::ptr_eq(original, realm))
    }

    /// Read original observations without discarding their inventory issuer.
    pub fn iter(&self) -> std::slice::Iter<'_, PathConstantWrite> {
        self.writes.iter()
    }
    /// Number of observations.
    #[must_use]
    pub fn len(&self) -> usize {
        self.writes.len()
    }
    /// Whether the inventory has no observations.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.writes.is_empty()
    }
    /// Reset the complete inventory, including its issuer and lexical scopes.
    pub fn clear(&mut self) {
        *self = Self::default();
    }
    /// Append a compatible original batch; incompatible issuers withdraw facts.
    pub fn append(&mut self, mut batch: Self) {
        if self.scopes.is_empty()
            && self.writes.is_empty()
            && self.policy.is_none()
            && self.barriers.is_empty()
        {
            *self = batch;
        } else if self.policy == batch.policy
            && self.source_config == batch.source_config
            && self.source_input == batch.source_input
            && self.source_image == batch.source_image
            && self.source_realm == batch.source_realm
        {
            self.recorded_end = self.recorded_end.max(batch.recorded_end);
            self.writes.append(&mut batch.writes);
            for scope in batch.scopes {
                if !self.scopes.contains(&scope) {
                    self.scopes.push(scope);
                }
            }
            self.barriers.append(&mut batch.barriers);
            self.unavailable_scopes
                .append(&mut batch.unavailable_scopes);
        } else {
            self.clear();
            self.barriers.push(0);
        }
    }
    /// Original accepted namespace body intervals for lexical navigation.
    pub fn namespace_body_spans(&self) -> impl Iterator<Item = tcl_lexer::Span> + '_ {
        self.scopes
            .iter()
            .filter(|scope| scope.id != 0)
            .map(|scope| tcl_lexer::Span::new(scope.start, scope.end))
    }
    /// Position-gated observations with the original issuer and scope intervals.
    #[must_use]
    pub fn before(&self, at: u32) -> Self {
        let mut prefix = self.clone();
        prefix.writes.retain(|write| write.at < at);
        prefix.barriers.retain(|barrier| *barrier < at);
        prefix
    }
}
impl std::ops::Index<usize> for PathConstantAssignments {
    type Output = PathConstantWrite;
    fn index(&self, index: usize) -> &Self::Output {
        &self.writes[index]
    }
}
impl<'a> IntoIterator for &'a PathConstantAssignments {
    type Item = &'a PathConstantWrite;
    type IntoIter = std::slice::Iter<'a, PathConstantWrite>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Folded values retain naming, original lexical scope and exportability.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FoldedPathConstants {
    policy: Option<NamePolicyProtocol>,
    globals: HashMap<String, String>,
    global_at: HashMap<String, u32>,
    locals: HashMap<(u32, String), String>,
    local_at: HashMap<(u32, String), u32>,
    aliases: HashMap<(u32, String), (String, u32)>,
    members: HashSet<(Home, String)>,
    scopes: Vec<Scope>,
    barriers: Vec<u32>,
    unavailable_scopes: Vec<(u32, u32)>,
}
impl FoldedPathConstants {
    /// Naming provider retained by this view.
    #[must_use]
    pub const fn naming_policy(&self) -> Option<NamePolicyProtocol> {
        self.policy
    }
    /// An empty view under an explicit issuer, including an unknown issuer.
    #[must_use]
    pub fn empty(policy: Option<NamePolicyProtocol>) -> Self {
        Self {
            policy,
            ..Self::default()
        }
    }
    /// Exact exported global key, without a written-name reinterpretation.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&String> {
        self.globals.get(key)
    }
    /// Exact exported global membership.
    #[must_use]
    pub fn contains_key(&self, key: &str) -> bool {
        self.globals.contains_key(key)
    }
    /// Whether there are no exported global values.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.globals.is_empty()
    }
    /// Number of exported global values.
    #[must_use]
    pub fn len(&self) -> usize {
        self.globals.len()
    }
    /// Exported global values only; ephemeral Jim locals do not escape.
    #[must_use]
    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, String, String> {
        self.globals.iter()
    }
    /// An import projection carrying the same issuer and only global homes.
    #[must_use]
    pub fn exported(&self) -> Self {
        Self {
            policy: self.policy,
            globals: if self.barriers.is_empty() {
                self.globals.clone()
            } else {
                HashMap::new()
            },
            ..Self::default()
        }
    }
    /// Agreement requires equal issuers as well as equal values on every route.
    #[must_use]
    pub fn agreement(views: &[Self]) -> Self {
        let Some(first) = views.first() else {
            return Self::default();
        };
        if views.iter().any(|view| view.policy != first.policy) {
            return Self::default();
        }
        let mut agreed = first.exported();
        agreed.globals.retain(|name, value| {
            views
                .iter()
                .all(|view| view.globals.get(name) == Some(value))
        });
        agreed
    }
    /// Bind lookup to an exact original source offset.
    #[must_use]
    pub fn at(&self, offset: u32) -> PathConstantView<'_> {
        PathConstantView {
            constants: self,
            offset,
        }
    }
    /// Original site lookup; accepted lexical intervals select local scope.
    #[must_use]
    pub fn lookup_at(&self, name: &str, at: u32) -> Option<String> {
        if self.barriers.iter().any(|barrier| *barrier <= at)
            || self
                .unavailable_scopes
                .iter()
                .any(|(start, end)| *start <= at && at < *end)
        {
            return None;
        }
        let scope = self
            .scopes
            .iter()
            .filter(|scope| scope.start <= at && at < scope.end)
            .min_by_key(|scope| scope.end - scope.start);
        self.lookup_in(name, scope, at)
    }
    fn lookup_in(&self, name: &str, scope: Option<&Scope>, at: u32) -> Option<String> {
        let protocol = self.policy?.recipe();
        let selected = protocol.variable_root_input(name.as_bytes());
        let name = std::str::from_utf8(selected.selected()).ok()?;
        if let Some(scope) = scope {
            if let Some((target, declared_at)) = self.aliases.get(&(scope.id, name.to_owned()))
                && *declared_at <= at
            {
                return self.global_value_at(target, at);
            }
            if scope.local && !name.starts_with("::") {
                let key = (scope.id, name.to_owned());
                return self
                    .local_at
                    .get(&key)
                    .filter(|written| **written <= at)
                    .and_then(|_| self.locals.get(&key))
                    .cloned();
            }
            let current = global_key(protocol, &scope.namespace, name);
            if !scope.namespace.is_empty() && !name.starts_with("::") {
                if self.members.contains(&(Home::Global, current.clone()))
                    || self.globals.contains_key(&current)
                {
                    return self.global_value_at(&current, at);
                }
                if matches!(protocol, NativeNameProtocol::C(version) if !version.namespace_var_global_fallback())
                {
                    return None;
                }
            }
        }
        self.global_value_at(&global_key(protocol, "", name), at)
    }
    fn global_value_at(&self, key: &str, at: u32) -> Option<String> {
        if self.global_at.get(key).is_some_and(|written| *written > at) {
            return None;
        }
        self.globals.get(key).cloned()
    }
}
impl<'a> IntoIterator for &'a FoldedPathConstants {
    type Item = (&'a String, &'a String);
    type IntoIter = std::collections::hash_map::Iter<'a, String, String>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl std::ops::Index<&str> for FoldedPathConstants {
    type Output = String;
    fn index(&self, index: &str) -> &Self::Output {
        &self.globals[index]
    }
}

/// A selected original source-site view of a folded environment.
pub struct PathConstantView<'a> {
    constants: &'a FoldedPathConstants,
    offset: u32,
}
impl PathConstantLookup for PathConstantView<'_> {
    fn path_constant(&self, name: &str) -> Option<String> {
        self.constants.lookup_at(name, self.offset)
    }
}

/// Read-only variable lookup accepted by the path-expression evaluator.
pub trait PathConstantLookup {
    /// Resolve a written name under the retained view's selected scope.
    fn path_constant(&self, name: &str) -> Option<String>;
}
impl PathConstantLookup for FoldedPathConstants {
    fn path_constant(&self, name: &str) -> Option<String> {
        self.lookup_at(name, u32::MAX - 1)
    }
}
impl<S: std::hash::BuildHasher> PathConstantLookup for HashMap<String, String, S> {
    fn path_constant(&self, name: &str) -> Option<String> {
        // Plain maps are an authored C abstraction, never native Jim inventories.
        if let Some(value) = self.get(name) {
            return Some(value.clone());
        }
        let rooted = super::canon(name);
        self.get(&rooted)
            .or_else(|| {
                tcl_syntax::naming::unroot_rooted_key(&rooted)
                    .filter(|bare| !bare.contains("::"))
                    .and_then(|bare| self.get(bare))
            })
            .cloned()
    }
}

/// Imported values carry their own naming authority at live consumer boundaries.
pub trait PathConstantImports {
    /// Validate the imported namespace naming against the receiving inventory.
    fn path_imports(&self, policy: NamePolicyProtocol) -> Option<HashMap<String, String>>;
}
impl PathConstantImports for FoldedPathConstants {
    fn path_imports(&self, policy: NamePolicyProtocol) -> Option<HashMap<String, String>> {
        if self.policy == Some(policy) || (self.policy.is_none() && self.globals.is_empty()) {
            Some(self.globals.clone())
        } else {
            None
        }
    }
}
impl<S: std::hash::BuildHasher> PathConstantImports for HashMap<String, String, S> {
    fn path_imports(&self, policy: NamePolicyProtocol) -> Option<HashMap<String, String>> {
        if self.is_empty()
            || (policy.authority() == tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation
                && matches!(policy.recipe(), NativeNameProtocol::C(_)))
        {
            Some(
                self.iter()
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect(),
            )
        } else {
            None
        }
    }
}

pub(super) fn authored_policy(profile: &'static DialectProfile) -> Option<NamePolicyProtocol> {
    tcl_registry::InvocationDialect::of_profile(profile).authored_name_policy()
}
fn global_key(protocol: NativeNameProtocol, namespace: &str, name: &str) -> String {
    match protocol {
        NativeNameProtocol::C(_) => {
            let key = super::qualified_key(namespace, name);
            if let Some(bare) =
                tcl_syntax::naming::unroot_rooted_key(&key).filter(|bare| !bare.contains("::"))
            {
                bare.to_owned()
            } else {
                key
            }
        }
        NativeNameProtocol::Jim084 => {
            let rooted = if namespace.is_empty() {
                "::"
            } else {
                namespace
            };
            tcl_syntax::naming::jim_global_variable_key(rooted, name)
        }
    }
}
fn root_namespace(namespace: &str) -> bool {
    namespace.is_empty() || tcl_syntax::naming::unroot_rooted_key(namespace) == Some("")
}
fn jim_namespace(protocol: NativeNameProtocol, namespace: &str, name: &str) -> Option<String> {
    let original = tcl_syntax::naming::unroot_rooted_key(namespace).unwrap_or(namespace);
    let root = tcl_core_types::ByteNamespacePath::root();
    let context = NativeNameContext::with_jim_namespace(&root, original.as_bytes());
    let input = protocol
        .jim_namespace_canonical_input(context, name.as_bytes())
        .ok()?;
    let key = std::str::from_utf8(input.selected()).ok()?;
    Some(tcl_syntax::naming::root_unrooted_key(key))
}

/// Inventory a complete authored document using only its independently selected naming recipe.
#[must_use]
pub fn constant_path_assignments(
    source: &str,
    profile: &'static DialectProfile,
) -> PathConstantAssignments {
    constant_path_assignments_from_commands(
        &super::segment_commands_with_offset_and_config(
            source,
            0,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
        ),
        profile,
    )
}
/// Inventory segmented original commands in the global lexical frame.
#[must_use]
pub fn constant_path_assignments_from_commands(
    commands: &[crate::segmenter::SegmentedCommand],
    profile: &'static DialectProfile,
) -> PathConstantAssignments {
    constant_path_assignments_in_namespace(commands, profile, None)
}
/// Inventory with an explicit constructed authored source-site namespace seed.
/// This is lexical advice, not proof of an entered native namespace or frame.
#[must_use]
pub fn constant_path_assignments_in_namespace(
    commands: &[crate::segmenter::SegmentedCommand],
    profile: &'static DialectProfile,
    namespace: Option<&str>,
) -> PathConstantAssignments {
    constant_path_assignments_with_naming_policy(
        commands,
        profile,
        authored_policy(profile),
        namespace,
    )
}

/// Inventory under an independently supplied naming provider. An explicit F5
/// simulation must pass the shared authored logical naming selector.
#[must_use]
pub fn constant_path_assignments_with_naming_policy(
    commands: &[crate::segmenter::SegmentedCommand],
    profile: &'static DialectProfile,
    policy: Option<NamePolicyProtocol>,
    namespace: Option<&str>,
) -> PathConstantAssignments {
    let dialect = tcl_registry::InvocationDialect::of_profile(profile);
    let policy = policy.filter(|provider| {
        dialect
            .authored_name_policy()
            .is_some_and(|selected| selected.recipe() == provider.recipe())
            || dialect
                .authored_logical_name_simulation(*provider)
                .is_some()
    });
    let mut out = PathConstantAssignments {
        policy,
        ..PathConstantAssignments::default()
    };
    let Some(policy) = policy else {
        return out;
    };
    let scope = Scope {
        id: 0,
        start: 0,
        end: u32::MAX,
        namespace: namespace.unwrap_or("").to_owned(),
        local: namespace.is_some() && policy.recipe() == NativeNameProtocol::Jim084,
    };
    out.scopes.push(scope.clone());
    collect(commands, profile, &scope, 0, &mut out);
    out
}

fn target(
    out: &PathConstantAssignments,
    scope: &Scope,
    name: &str,
    declares: bool,
) -> Option<(Home, String, Option<String>)> {
    let protocol = out.policy?.recipe();
    let selected = protocol.variable_root_input(name.as_bytes());
    let name = std::str::from_utf8(selected.selected()).ok()?;
    if declares && protocol == NativeNameProtocol::Jim084 {
        let tail = tcl_syntax::naming::variable_local_name_bytes(protocol, name.as_bytes());
        let tail = String::from_utf8(tail).ok()?;
        if scope.local || !scope.namespace.is_empty() {
            return Some((
                Home::Global,
                global_key(
                    protocol,
                    "",
                    &jim_namespace(protocol, &scope.namespace, name)?,
                ),
                Some(tail),
            ));
        }
        return Some((Home::Global, tail, None));
    }
    if let Some(alias) = out
        .writes
        .iter()
        .rev()
        .find(|write| write.scope == scope.id && write.alias.as_deref() == Some(name))
    {
        return Some((alias.home, alias.name.clone(), None));
    }
    if scope.local && !name.starts_with("::") && !declares {
        return Some((Home::Local(scope.id), name.to_owned(), None));
    }
    let current = global_key(protocol, &scope.namespace, name);
    if declares && (scope.id != 0 || !scope.namespace.is_empty()) {
        let tail = tcl_syntax::naming::variable_local_name_bytes(protocol, name.as_bytes());
        return Some((Home::Global, current, Some(String::from_utf8(tail).ok()?)));
    }
    if declares
        || root_namespace(&scope.namespace)
        || name.starts_with("::")
        || matches!(protocol, NativeNameProtocol::C(version) if !version.namespace_var_global_fallback())
    {
        return Some((Home::Global, current, None));
    }
    if out
        .writes
        .iter()
        .any(|write| write.home == Home::Global && write.name == current)
    {
        return Some((Home::Global, current, None));
    }
    let global = global_key(protocol, "", name);
    if out
        .writes
        .iter()
        .any(|write| write.home == Home::Global && write.name == global)
    {
        return Some((Home::Global, global, None));
    }
    None
}
#[derive(Clone, Copy)]
struct PathCollectionContext<'a> {
    profile: &'static DialectProfile,
    scope: &'a Scope,
    base: u32,
    at: u32,
}

fn collect(
    commands: &[crate::segmenter::SegmentedCommand],
    profile: &'static DialectProfile,
    scope: &Scope,
    base: u32,
    out: &mut PathConstantAssignments,
) {
    let generation = crate::environment_ingress::context_for_profile(profile);
    let registry = generation.commands();
    for command in commands {
        let Some(head) = command.texts.first() else {
            continue;
        };
        let Some(at) = base.checked_add(command.span.start()) else {
            out.barriers.push(0);
            continue;
        };
        let head = head.strip_prefix("::").unwrap_or(head);
        if head.contains('$') || head.contains('[') || command.is_partial {
            out.barriers.push(at);
            continue;
        }
        let args: Vec<&str> = command.texts[1..].iter().map(String::as_str).collect();
        let context = PathCollectionContext {
            profile,
            scope,
            base,
            at,
        };
        if let Some(body) = namespace_body_index(registry, head, &args) {
            collect_namespace_body(command, &args, registry, head, body, &context, out);
            continue;
        }
        for body in registry.arg_indices_for_role(head, &args, tcl_registry::ArgRole::Body) {
            if let Some(token) = command.argv.get(body + 1)
                && let (Some(start), Some(end)) = (
                    base.checked_add(token.span.start()),
                    base.checked_add(token.span.end()),
                )
            {
                out.unavailable_scopes.push((start, end));
            }
        }
        if registry.get(head).is_none_or(|spec| {
            matches!(
                spec.lowering_hook,
                Some(
                    tcl_registry::hooks::LoweringHookId::Eval
                        | tcl_registry::hooks::LoweringHookId::Uplevel
                        | tcl_registry::hooks::LoweringHookId::Apply
                        | tcl_registry::hooks::LoweringHookId::NamespaceEval
                )
            )
        }) {
            out.barriers.push(at);
        }
        let pairs = assigned_name_value_indices(registry, head, &args);
        let declares = registry.get(head).is_some_and(|spec| {
            spec.traits
                .contains(tcl_registry::Traits::CREATES_SCOPE_ALIAS)
        });
        if !pairs.is_empty() {
            collect_assignments(command, &args, pairs, declares, &context, out);
            continue;
        }
        if declares {
            out.barriers.push(at);
            continue;
        }
        for index in registry.arg_indices_for_role(head, &args, tcl_registry::ArgRole::VarWrite) {
            let Some(name) = args.get(index).filter(|name| is_plain_scalar_name(name)) else {
                out.barriers.push(at);
                continue;
            };
            if let Some((home, name, alias)) = target(out, scope, name, false) {
                out.writes.push(PathConstantWrite {
                    name,
                    ns: scope.namespace.clone(),
                    at,
                    value: PathConstantValue::Poisoned,
                    home,
                    scope: scope.id,
                    alias,
                });
            } else {
                out.barriers.push(at);
            }
        }
    }
}

fn collect_namespace_body(
    command: &crate::segmenter::SegmentedCommand,
    args: &[&str],
    registry: &tcl_registry::CommandRegistry,
    head: &str,
    body: usize,
    context: &PathCollectionContext<'_>,
    out: &mut PathConstantAssignments,
) {
    let PathCollectionContext {
        profile,
        scope,
        base,
        at,
    } = *context;
    let names = registry.arg_indices_for_role(head, args, tcl_registry::ArgRole::NamespaceName);
    let name = names.first().and_then(|index| args.get(*index));
    let token = command.argv.get(body + 1);
    if let (Some(name), Some(token)) = (name, token)
        && is_plain_scalar_name(name)
        && token.kind == tcl_lexer::TokenType::Str
        && command.single_token_word.get(body + 1) == Some(&true)
    {
        let protocol = out.policy.expect("selected inventory").recipe();
        let namespace = match protocol {
            NativeNameProtocol::C(_) => {
                Some(crate::naming::qualify_namespace(&scope.namespace, name))
            }
            NativeNameProtocol::Jim084 => jim_namespace(protocol, &scope.namespace, name),
        };
        if let Some(namespace) = namespace {
            let Some(start) = base
                .checked_add(token.span.start())
                .and_then(|start| start.checked_add(u32::from(token.content_offset)))
            else {
                out.barriers.push(at);
                return;
            };
            let text = &command.texts[body + 1];
            let Ok(length) = u32::try_from(text.len()) else {
                out.barriers.push(at);
                return;
            };
            let Some(end) = start.checked_add(length) else {
                out.barriers.push(at);
                return;
            };
            let child = Scope {
                id: start,
                start,
                end,
                namespace,
                local: protocol == NativeNameProtocol::Jim084,
            };
            out.scopes.push(child.clone());
            collect(
                &super::segment_commands_with_offset_and_config(
                    text,
                    0,
                    tcl_lexer::LexerConfig::from_grammar(profile.grammar),
                ),
                profile,
                &child,
                start,
                out,
            );
            return;
        }
    }
    out.barriers.push(at);
}

fn collect_assignments(
    command: &crate::segmenter::SegmentedCommand,
    args: &[&str],
    pairs: Vec<(usize, Option<usize>)>,
    declares: bool,
    context: &PathCollectionContext<'_>,
    out: &mut PathConstantAssignments,
) {
    let scope = context.scope;
    let at = context.at;
    for (name_index, value_index) in pairs {
        if args
            .get(name_index)
            .is_some_and(|name| name.contains('(') && !name.contains('$') && !name.contains('['))
        {
            continue;
        }
        let Some(name) = args
            .get(name_index)
            .filter(|name| is_plain_scalar_name(name))
        else {
            out.barriers.push(at);
            continue;
        };
        let value = value_index
            .filter(|index| *index < args.len())
            .map_or(PathConstantValue::Declared, |index| {
                raw_value(command, index + 1)
            });
        if let Some((home, name, alias)) = target(out, scope, name, declares) {
            out.writes.push(PathConstantWrite {
                name,
                ns: scope.namespace.clone(),
                at,
                value,
                home,
                scope: scope.id,
                alias,
            });
        } else {
            let protocol = out.policy.expect("selected inventory").recipe();
            for namespace in [&scope.namespace[..], ""] {
                let name = global_key(protocol, namespace, name);
                out.writes.push(PathConstantWrite {
                    name,
                    ns: scope.namespace.clone(),
                    at,
                    value: PathConstantValue::Poisoned,
                    home: Home::Global,
                    scope: scope.id,
                    alias: None,
                });
            }
        }
    }
}

/// Fold original observations, retaining the issuer and original lexical scopes.
#[must_use]
pub fn fold_constant_assignments(
    assignments: &PathConstantAssignments,
    info_script: Option<&str>,
) -> FoldedPathConstants {
    fold_constant_assignments_with_imports(
        assignments,
        info_script,
        &FoldedPathConstants::default(),
    )
}
/// Fold with compatible global-home imports; incompatible policies withdraw the view.
#[must_use]
pub fn fold_constant_assignments_with_imports<I: PathConstantImports + ?Sized>(
    assignments: &PathConstantAssignments,
    info_script: Option<&str>,
    imported: &I,
) -> FoldedPathConstants {
    let Some(policy) = assignments.policy else {
        return FoldedPathConstants::default();
    };
    let Some(globals) = imported.path_imports(policy) else {
        return FoldedPathConstants::default();
    };
    let mut out = FoldedPathConstants {
        policy: Some(policy),
        globals,
        scopes: assignments.scopes.clone(),
        barriers: assignments.barriers.clone(),
        unavailable_scopes: assignments.unavailable_scopes.clone(),
        ..FoldedPathConstants::default()
    };
    let mut counts: HashMap<(Home, String), usize> = HashMap::new();
    for write in assignments {
        out.members.insert((write.home, write.name.clone()));
        let count = counts.entry((write.home, write.name.clone())).or_default();
        *count += match write.value {
            PathConstantValue::Raw { .. } | PathConstantValue::Original(_) => 1,
            PathConstantValue::Poisoned => 2,
            PathConstantValue::Declared => 0,
        };
    }
    for ((home, name), count) in &counts {
        if *count != 0 && *home == Home::Global {
            out.globals.remove(name);
        }
    }
    for write in assignments {
        if let Some(alias) = &write.alias {
            out.aliases
                .insert((write.scope, alias.clone()), (write.name.clone(), write.at));
        }
        if counts.get(&(write.home, write.name.clone())) != Some(&1) {
            continue;
        }
        let value = match &write.value {
            PathConstantValue::Original(expression) => {
                expression.evaluate(info_script, &|name| out.lookup_at(name, write.at))
            }
            PathConstantValue::Raw { text, literal } => {
                if !literal && super::carries_substitution(text) {
                    super::evaluate_auto_path_expr_with_resolver_and_braced_vars(
                        text,
                        info_script,
                        &|name| out.lookup_at(name, write.at),
                        assignments
                            .source_config
                            .map_or_else(tcl_dialect::BracedVarStyle::default, |config| {
                                config.braced_var
                            }),
                    )
                } else {
                    Some(text.clone())
                }
            }
            _ => continue,
        };
        if let Some(value) = value {
            match write.home {
                Home::Global => {
                    out.global_at.insert(write.name.clone(), write.at);
                    out.globals.insert(write.name.clone(), value);
                }
                Home::Local(id) => {
                    out.local_at.insert((id, write.name.clone()), write.at);
                    out.locals.insert((id, write.name.clone()), value);
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(name: &str) -> &'static DialectProfile {
        tcl_registry::model::ingress::resolve_environment(name).analyser_profile()
    }
    fn view(source: &str, name: &str) -> FoldedPathConstants {
        fold_constant_assignments(&constant_path_assignments(source, profile(name)), None)
    }

    #[test]
    fn original_native_scope_names_keep_six_engine_colon_and_home_rules() {
        // Native proof naming.namespace.scope-repeated-colon-home:
        // docs/design/analysis/name-resolution-proofs/namespace-scope-repeated-colon-home.md
        // Native proof naming.variable.scope-repeated-colon-written-key:
        // docs/design/analysis/name-resolution-proofs/variable-scope-repeated-colon-written-key.md
        let source = include_str!("../../tests/data/native_path_constant_scopes/colon-home.tcl");
        let raw =
            include_str!("../../tests/data/native_path_constant_scopes/raw-qualified-key.tcl");
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let constants = view(source, engine);
            let at = u32::try_from(source.len() - 1).unwrap();
            assert_eq!(
                constants.lookup_at("::a:::b::dir", at).as_deref(),
                Some("/LIB"),
                "{engine}"
            );
            assert_eq!(
                constants.lookup_at("::a::b::dir", at).is_some(),
                engine != "jim",
                "{engine}"
            );
            let constants = view(raw, engine);
            let at = u32::try_from(raw.len() - 1).unwrap();
            assert_eq!(
                constants.lookup_at("a:::raw", at).as_deref(),
                Some("/ONE"),
                "{engine}"
            );
            assert_eq!(
                constants.lookup_at("a::raw", at).is_some(),
                engine != "jim",
                "{engine}"
            );
        }
    }

    #[test]
    fn original_native_jim_qualified_variable_at_root_writes_only_local_tail() {
        // Native proof naming.variable.scope-root-qualified-declaration:
        // docs/design/analysis/name-resolution-proofs/variable-scope-root-qualified-declaration.md
        let source = include_str!(
            "../../tests/data/native_path_constant_scopes/root-qualified-variable.tcl"
        );
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let constants = view(source, engine);
            let at = u32::try_from(source.len() - 1).unwrap();
            assert_eq!(
                constants.lookup_at("x", at).as_deref(),
                (engine == "jim").then_some("/TOP"),
                "{engine}"
            );
            assert_eq!(
                constants.lookup_at("::a::x", at).as_deref(),
                (engine != "jim").then_some("/TOP"),
                "{engine}"
            );
        }
    }

    #[test]
    fn original_native_root_namespace_activation_does_not_export_jim_locals() {
        // Native proof naming.variable.scope-root-namespace-activation:
        // docs/design/analysis/name-resolution-proofs/variable-scope-root-namespace-activation.md
        let source =
            include_str!("../../tests/data/native_path_constant_scopes/root-activation.tcl");
        let inside = u32::try_from(source.find("puts \"in:").unwrap()).unwrap();
        let outside = u32::try_from(source.find("puts \"out:").unwrap()).unwrap();
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let constants = view(source, engine);
            assert_eq!(
                constants.lookup_at("ephemeral", inside).as_deref(),
                Some("/LOCAL"),
                "{engine}"
            );
            assert_eq!(
                constants.lookup_at("ephemeral", outside).is_some(),
                engine != "jim",
                "{engine}"
            );
            assert_eq!(
                constants.exported().contains_key("ephemeral"),
                engine != "jim",
                "{engine}"
            );
        }
    }

    #[test]
    fn namespace_read_sites_retain_c8_c9_and_jim_receiver_channels() {
        // Native proof naming.variable.scope-namespace-local-and-linked-publication:
        // docs/design/analysis/name-resolution-proofs/variable-scope-namespace-local-and-linked-publication.md
        let source =
            include_str!("../../tests/data/native_path_constant_scopes/local-and-linked.tcl");
        let inside = u32::try_from(source.find("puts \"local:").unwrap()).unwrap();
        let outside = u32::try_from(source.find("puts \"global:").unwrap()).unwrap();
        // The native C8 source writes the same global twice; the static
        // single-assignment abstraction withdraws both reads rather than
        // choosing one of those values.
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6"] {
            let constants = view(source, engine);
            assert_eq!(constants.lookup_at("dir", inside), None, "{engine}");
            assert_eq!(constants.lookup_at("dir", outside), None, "{engine}");
        }
        for engine in ["tcl9.0", "tcl9.1", "jim"] {
            let constants = view(source, engine);
            assert_eq!(
                constants.lookup_at("dir", inside).as_deref(),
                Some("/LOCAL"),
                "{engine}"
            );
            assert_eq!(
                constants.lookup_at("dir", outside).as_deref(),
                Some("/GLOBAL"),
                "{engine}"
            );
            assert_eq!(
                constants.lookup_at("::N::dir", outside).is_some(),
                engine != "jim",
                "{engine}"
            );
        }
    }

    #[test]
    fn typed_empty_prefix_and_agreement_never_erase_naming_issuer() {
        let c = constant_path_assignments("set dir /C", profile("tcl8.6"));
        let jim = constant_path_assignments("set dir /J", profile("jim"));
        assert!(c.before(0).is_empty());
        assert_eq!(c.before(0).naming_policy(), c.naming_policy());
        assert_ne!(c.before(0), jim.before(0));
        let mut batches = c.before(0);
        batches.append(c.clone());
        assert_eq!(batches, c);
        batches.append(jim.clone());
        assert_eq!(batches.naming_policy(), None);
        let c = fold_constant_assignments(&c, None);
        let jim = fold_constant_assignments(&jim, None);
        assert_eq!(
            FoldedPathConstants::agreement(&[c.clone(), jim.clone()]).naming_policy(),
            None
        );
        assert!(
            fold_constant_assignments_with_imports(
                &constant_path_assignments("set other /X", profile("jim")),
                None,
                &c
            )
            .is_empty()
        );
        assert!(FoldedPathConstants::agreement(&[jim.clone(), jim]).contains_key("dir"));
    }

    #[test]
    fn original_jim_scope_intervals_do_not_join_equal_namespace_activations() {
        let source = "namespace eval N {set dir /FIRST; source $dir/a.tcl}; namespace eval N {source $dir/b.tcl}";
        let constants = view(source, "jim");
        let first = u32::try_from(source.find("source").unwrap()).unwrap();
        let second = u32::try_from(source.rfind("source").unwrap()).unwrap();
        assert_eq!(constants.lookup_at("dir", first).as_deref(), Some("/FIRST"));
        assert_eq!(constants.lookup_at("dir", second), None);
        assert!(constants.exported().is_empty());
    }

    #[test]
    fn unknown_f5_provider_and_dynamic_scope_changes_withdraw_path_facts() {
        let f5 = constant_path_assignments("set dir /F5", profile("f5-irules"));
        assert_eq!(f5.naming_policy(), None);
        assert!(fold_constant_assignments(&f5, None).is_empty());
        for source in [
            "set dir /OLD; namespace eval $ns {set dir /NEW}; source $dir/a.tcl",
            "set dir /OLD; upvar 0 other dir; source $dir/a.tcl",
            "set dir /OLD; $operation; source $dir/a.tcl",
        ] {
            let constants = view(source, "tcl8.6");
            assert_eq!(
                constants.lookup_at(
                    "dir",
                    u32::try_from(source.rfind("source").unwrap()).unwrap()
                ),
                None,
                "{source}"
            );
            assert!(constants.exported().is_empty(), "{source}");
        }
    }
    #[test]
    fn original_site_lookup_never_borrows_a_later_write_or_unmodelled_body_scope() {
        let source = "source $dir/early.tcl; set dir /AFTER; proc p {} {source $dir/local.tcl}; source $dir/late.tcl";
        let constants = view(source, "tcl8.6");
        assert_eq!(constants.lookup_at("dir", 0), None);
        assert_eq!(
            constants.lookup_at(
                "dir",
                u32::try_from(source.find("source $dir/local").unwrap()).unwrap()
            ),
            None
        );
        assert_eq!(
            constants
                .lookup_at(
                    "dir",
                    u32::try_from(source.rfind("source").unwrap()).unwrap()
                )
                .as_deref(),
            Some("/AFTER")
        );
    }

    #[test]
    fn explicit_f5_naming_provider_is_separate_from_default_unknown_inventory() {
        let profile = profile("f5-irules");
        let commands = super::super::segment_commands_with_offset_and_config(
            "set dir /AUTHORED",
            0,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
        );
        assert_eq!(
            constant_path_assignments_from_commands(&commands, profile).naming_policy(),
            None
        );
        let provider = NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_4);
        let inventory =
            constant_path_assignments_with_naming_policy(&commands, profile, Some(provider), None);
        assert_eq!(inventory.naming_policy(), Some(provider));
        assert_eq!(
            fold_constant_assignments(&inventory, None)
                .get("dir")
                .map(String::as_str),
            Some("/AUTHORED")
        );
        let wrong = constant_path_assignments_with_naming_policy(
            &commands,
            profile,
            Some(NamePolicyProtocol::authored_jim084()),
            None,
        );
        assert_eq!(wrong.naming_policy(), None);
    }
}
