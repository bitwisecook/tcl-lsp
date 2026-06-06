//! Command namespaces (T1.5) — the command-table-as-core-service.
//!
//! The runtime's command lookup is **one** `resolve(currentNs, name) → Command`
//! function (the command-binding contract's A1/A2 — see
//! `docs/design/contracts/command-binding-and-aliasing.md`):
//!
//! 1. Parse the name: a leading or embedded `::` ⇒ **qualified** (absolute if
//!    leading `::`, else relative to `currentNs`) → look up the simple name
//!    directly in that namespace (no path/import fallback).
//! 2. **Unqualified** → **(a)** the current namespace's command table, **(b)**
//!    each namespace on its `namespace path` in order, **(c)** the global `::`,
//!    **(d)** miss (the caller raises `invalid command name`, later `unknown`).
//!
//! The tree is an arena (`Vec<Namespace>` + [`NsId`] indices) — no `Rc`/parent
//! pointers, `wasm32`-friendly. `rename`/`interp alias`/`import`/ensembles layer
//! on this one resolver (they install redirect/alias `Command`s); the binding
//! lattice (only `pristine-builtin` inlines) is the AOT side.

use std::collections::BTreeMap;

use crate::interp::Command;

/// An index into the namespace arena. The global namespace `::` is always 0.
pub type NsId = usize;

/// The global namespace `::`.
pub const GLOBAL: NsId = 0;

/// The result of a [`Namespaces::rename`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenameOutcome {
    /// `old` was moved to `new`.
    Renamed,
    /// `rename old ""` removed the command.
    Deleted,
    /// `old` did not resolve to any command.
    NoSuchCommand,
}

/// One namespace: its simple name, its command table, child namespaces, the
/// `namespace path` search list, and `export` patterns.
struct Namespace {
    /// Simple name (e.g. `mathfunc`); the global namespace's is empty.
    name: Vec<u8>,
    parent: Option<NsId>,
    children: BTreeMap<Vec<u8>, NsId>,
    commands: BTreeMap<Vec<u8>, Command>,
    /// `namespace path` — namespaces searched for unqualified commands (step b).
    path: Vec<NsId>,
    /// `namespace export` patterns — gate what `import` may pull (matched with
    /// `string match` glob via the shared [`tcl_syntax::glob`]).
    exports: Vec<Vec<u8>>,
}

impl Namespace {
    fn new(name: Vec<u8>, parent: Option<NsId>) -> Namespace {
        Namespace {
            name,
            parent,
            children: BTreeMap::new(),
            commands: BTreeMap::new(),
            path: Vec::new(),
            exports: Vec::new(),
        }
    }
}

/// The namespace tree + the command resolver.
pub struct Namespaces {
    arena: Vec<Namespace>,
}

impl Default for Namespaces {
    fn default() -> Self {
        Self::new()
    }
}

impl Namespaces {
    /// A fresh tree with just the global namespace `::`.
    #[must_use]
    pub fn new() -> Namespaces {
        Namespaces {
            arena: vec![Namespace::new(Vec::new(), None)],
        }
    }

    /// Register `command` under `name` (possibly qualified), creating any
    /// intermediate namespaces. A qualified `name` is rooted at global.
    pub fn register(&mut self, name: &[u8], command: Command) {
        let segments = split_qualifier(name);
        let Some((simple, ns_parts)) = segments.split_last() else {
            return; // empty / `::` only — nothing to register
        };
        let mut ns = GLOBAL;
        for part in ns_parts {
            ns = self.ensure_child(ns, part);
        }
        self.arena[ns].commands.insert((*simple).to_vec(), command);
    }

    /// The single command resolver, `resolve(currentNs, name)` (A2). Returns a
    /// **clone** of the small command handle (a fn-pointer for `Builtin`; the
    /// target + frozen prefix for `Alias`) so the caller can dispatch without
    /// holding a borrow on the table.
    #[must_use]
    pub fn resolve(&self, current: NsId, name: &[u8]) -> Option<Command> {
        self.home_of(current, name)
            .map(|(ns, simple)| self.arena[ns].commands[&simple].clone())
    }

    /// Sorted command names in namespace `ns` (`info commands`).
    #[must_use]
    pub fn command_names(&self, ns: NsId) -> Vec<&[u8]> {
        self.arena[ns].commands.keys().map(Vec::as_slice).collect()
    }

    /// Remove the command bound to `name` (the `rename old ""` / alias-clear
    /// path); returns whether it existed. Honours the full resolution order so
    /// `delete` retires the same binding `resolve` would have hit.
    pub fn delete(&mut self, current: NsId, name: &[u8]) -> bool {
        match self.home_of(current, name) {
            Some((ns, simple)) => self.arena[ns].commands.remove(&simple).is_some(),
            None => false,
        }
    }

    /// `rename old new`: move the command bound to `old` to `new` (both resolved
    /// relative to `current`, absolute when `::`-led); `new == ""` deletes it.
    /// A self-rename is a no-op (remove-then-reinsert under the same key).
    ///
    /// Built-in protection lives in the `rename` builtin, not here — this is the
    /// pure table operation.
    pub fn rename(&mut self, current: NsId, old: &[u8], new: &[u8]) -> RenameOutcome {
        let Some((old_ns, old_simple)) = self.home_of(current, old) else {
            return RenameOutcome::NoSuchCommand;
        };
        // SAFETY of unwrap: `home_of` reported the binding exists.
        let cmd = self.arena[old_ns].commands.remove(&old_simple).unwrap();
        if new.is_empty() {
            return RenameOutcome::Deleted;
        }
        let absolute = new.starts_with(b"::");
        let segments = split_qualifier(new);
        let Some((simple, ns_parts)) = segments.split_last() else {
            // `new` was `::`-only — nothing nameable; put the command back.
            self.arena[old_ns].commands.insert(old_simple, cmd);
            return RenameOutcome::NoSuchCommand;
        };
        let mut ns = if absolute { GLOBAL } else { current };
        for part in ns_parts {
            ns = self.ensure_child(ns, part);
        }
        self.arena[ns].commands.insert((*simple).to_vec(), cmd);
        RenameOutcome::Renamed
    }

    /// Every alias command's fully-qualified name across the tree (`interp
    /// aliases`). Global aliases keep their simple name (aliases are registered
    /// interpreter-wide); namespaced ones are qualified.
    #[must_use]
    pub fn alias_names(&self) -> Vec<Vec<u8>> {
        let mut found: Vec<(NsId, Vec<u8>)> = Vec::new();
        for (id, ns) in self.arena.iter().enumerate() {
            for (key, cmd) in &ns.commands {
                if matches!(cmd, Command::Alias { .. }) {
                    found.push((id, key.clone()));
                }
            }
        }
        found
            .into_iter()
            .map(|(id, key)| {
                if id == GLOBAL {
                    key
                } else {
                    let mut q = self.qualified_name(id);
                    q.extend_from_slice(b"::");
                    q.extend_from_slice(&key);
                    q
                }
            })
            .collect()
    }

    /// Set namespace `ns`'s `namespace path` to the given namespaces.
    pub fn set_path(&mut self, ns: NsId, path: Vec<NsId>) {
        self.arena[ns].path = path;
    }

    /// Find (creating if needed) the namespace named `qualified`, rooted at
    /// `current` (absolute if it leads with `::`). For `namespace eval`.
    pub fn ensure_namespace(&mut self, current: NsId, qualified: &[u8]) -> NsId {
        let absolute = qualified.starts_with(b"::");
        let mut ns = if absolute { GLOBAL } else { current };
        for part in split_qualifier(qualified) {
            ns = self.ensure_child(ns, part);
        }
        ns
    }

    /// Resolve `qualified` to an existing namespace, or `None`.
    #[must_use]
    pub fn find_namespace(&self, current: NsId, qualified: &[u8]) -> Option<NsId> {
        let absolute = qualified.starts_with(b"::");
        let mut ns = if absolute { GLOBAL } else { current };
        for part in split_qualifier(qualified) {
            ns = *self.arena[ns].children.get(part)?;
        }
        Some(ns)
    }

    /// The fully-qualified name of `ns` (`::a::b`; global is `::`).
    #[must_use]
    pub fn qualified_name(&self, ns: NsId) -> Vec<u8> {
        let mut parts: Vec<&[u8]> = Vec::new();
        let mut cur = ns;
        while let Some(parent) = self.arena[cur].parent {
            parts.push(&self.arena[cur].name);
            cur = parent;
        }
        let mut out = Vec::new();
        if parts.is_empty() {
            return b"::".to_vec();
        }
        for part in parts.iter().rev() {
            out.extend_from_slice(b"::");
            out.extend_from_slice(part);
        }
        out
    }

    // -- the `namespace` command surface --------------------------------------

    /// The fully-qualified name a command `name` resolves to from `current`
    /// (`namespace which -command`), or `None` if it doesn't resolve.
    #[must_use]
    pub fn which_command(&self, current: NsId, name: &[u8]) -> Option<Vec<u8>> {
        let (ns, simple) = self.home_of(current, name)?;
        let mut fqn = self.qualified_name(ns);
        if ns != GLOBAL {
            fqn.extend_from_slice(b"::"); // global's qualified_name is already `::`
        }
        fqn.extend_from_slice(&simple);
        Some(fqn)
    }

    /// `namespace export` — append a pattern (deduplicated). `-clear` first is the
    /// caller's job via [`clear_exports`](Self::clear_exports).
    pub fn export(&mut self, ns: NsId, pattern: &[u8]) {
        if !self.arena[ns].exports.iter().any(|p| p == pattern) {
            self.arena[ns].exports.push(pattern.to_vec());
        }
    }

    /// Drop all of `ns`'s export patterns (`namespace export -clear`).
    pub fn clear_exports(&mut self, ns: NsId) {
        self.arena[ns].exports.clear();
    }

    /// `ns`'s export patterns (`namespace export` with no args).
    #[must_use]
    pub fn exports(&self, ns: NsId) -> &[Vec<u8>] {
        &self.arena[ns].exports
    }

    /// Does `name` match any of `ns`'s export patterns (`string match` glob)?
    #[must_use]
    pub fn is_exported(&self, ns: NsId, name: &[u8]) -> bool {
        let Ok(name_s) = core::str::from_utf8(name) else {
            return false;
        };
        self.arena[ns].exports.iter().any(|pat| {
            core::str::from_utf8(pat).is_ok_and(|p| tcl_syntax::glob::string_match(p, name_s))
        })
    }

    /// Bind `command` under simple `name` directly in namespace `ns` (no
    /// qualified parsing — `import` inserting a redirect into the importing ns).
    pub fn bind(&mut self, ns: NsId, name: &[u8], command: Command) {
        self.arena[ns].commands.insert(name.to_vec(), command);
    }

    /// `ns`'s `namespace path` (the resolver's step-b list).
    #[must_use]
    pub fn path(&self, ns: NsId) -> &[NsId] {
        &self.arena[ns].path
    }

    /// `ns`'s parent (`namespace parent`); `None` only for the global root.
    #[must_use]
    pub fn parent(&self, ns: NsId) -> Option<NsId> {
        self.arena[ns].parent
    }

    /// `ns`'s direct child namespaces (`namespace children`).
    #[must_use]
    pub fn children(&self, ns: NsId) -> Vec<NsId> {
        self.arena[ns].children.values().copied().collect()
    }

    /// The `(simple_name, source_fqn)` of every imported redirect in `ns`
    /// (`namespace forget` walks these).
    #[must_use]
    pub fn imported_in(&self, ns: NsId) -> Vec<(Vec<u8>, Vec<u8>)> {
        self.arena[ns]
            .commands
            .iter()
            .filter_map(|(k, c)| match c {
                Command::Imported { source } => Some((k.clone(), source.clone())),
                _ => None,
            })
            .collect()
    }

    /// Remove the simple-named command `name` directly from `ns` (no resolution
    /// walk); returns whether it existed. For `namespace forget`.
    pub fn remove_in(&mut self, ns: NsId, name: &[u8]) -> bool {
        self.arena[ns].commands.remove(name).is_some()
    }

    // -- helpers --------------------------------------------------------------

    /// Locate the namespace + simple name that *holds* the binding `name`
    /// resolves to, following the full resolution order (qualified → current →
    /// path → global). The shared core of `resolve`/`delete`/`rename`.
    fn home_of(&self, current: NsId, name: &[u8]) -> Option<(NsId, Vec<u8>)> {
        if let Some((ns, simple)) = self.resolve_qualified(current, name) {
            return self.arena[ns]
                .commands
                .contains_key(simple)
                .then(|| (ns, simple.to_vec()));
        }
        if self.arena[current].commands.contains_key(name) {
            return Some((current, name.to_vec()));
        }
        for &p in &self.arena[current].path {
            if self.arena[p].commands.contains_key(name) {
                return Some((p, name.to_vec()));
            }
        }
        if current != GLOBAL && self.arena[GLOBAL].commands.contains_key(name) {
            return Some((GLOBAL, name.to_vec()));
        }
        None
    }

    fn ensure_child(&mut self, parent: NsId, name: &[u8]) -> NsId {
        if let Some(&id) = self.arena[parent].children.get(name) {
            return id;
        }
        let id = self.arena.len();
        self.arena.push(Namespace::new(name.to_vec(), Some(parent)));
        self.arena[parent].children.insert(name.to_vec(), id);
        id
    }

    /// If `name` is qualified, walk to its namespace and return
    /// `(target_ns, simple_name)`; else `None` (unqualified).
    fn resolve_qualified<'n>(&self, current: NsId, name: &'n [u8]) -> Option<(NsId, &'n [u8])> {
        if !contains_qualifier(name) {
            return None;
        }
        let absolute = name.starts_with(b"::");
        let segments = split_qualifier(name);
        let (simple, ns_parts) = segments.split_last()?;
        let mut ns = if absolute { GLOBAL } else { current };
        for part in ns_parts {
            ns = *self.arena[ns].children.get(*part)?;
        }
        Some((ns, simple))
    }
}

/// Does `name` contain a `::` namespace separator?
fn contains_qualifier(name: &[u8]) -> bool {
    name.windows(2).any(|w| w == b"::")
}

/// Split a (possibly qualified) name on `::`, dropping empty segments — so
/// `::a::b::cmd` → `[a, b, cmd]`, `::cmd` → `[cmd]`, `cmd` → `[cmd]`, `::` → `[]`.
fn split_qualifier(name: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    let mut seg_start = 0;
    let mut i = 0;
    while i < name.len() {
        if i + 1 < name.len() && name[i] == b':' && name[i + 1] == b':' {
            if i > seg_start {
                out.push(&name[seg_start..i]);
            }
            i += 2;
            seg_start = i;
        } else {
            i += 1;
        }
    }
    if seg_start < name.len() {
        out.push(&name[seg_start..]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interp::Code;

    fn dummy(_i: &mut crate::interp::Interp, _a: &[*mut crate::obj::TclObj]) -> Code {
        Code::Ok
    }
    fn cmd() -> Command {
        Command::Builtin(dummy)
    }
    fn is_some(c: Option<Command>) -> bool {
        c.is_some()
    }

    #[test]
    fn split_qualifier_cases() {
        assert_eq!(
            split_qualifier(b"::a::b::cmd"),
            vec![&b"a"[..], b"b", b"cmd"]
        );
        assert_eq!(split_qualifier(b"::cmd"), vec![&b"cmd"[..]]);
        assert_eq!(split_qualifier(b"cmd"), vec![&b"cmd"[..]]);
        assert_eq!(split_qualifier(b"a::b"), vec![&b"a"[..], b"b"]);
        assert!(split_qualifier(b"::").is_empty());
    }

    #[test]
    fn unqualified_resolves_in_global() {
        let mut ns = Namespaces::new();
        ns.register(b"set", cmd());
        assert!(is_some(ns.resolve(GLOBAL, b"set")));
        assert!(!is_some(ns.resolve(GLOBAL, b"nope")));
    }

    #[test]
    fn qualified_registration_and_resolution() {
        let mut ns = Namespaces::new();
        ns.register(b"::tcl::mathfunc::sin", cmd());
        ns.register(b"set", cmd());
        let mf = ns.find_namespace(GLOBAL, b"::tcl::mathfunc").unwrap();
        // qualified lookup hits the namespace directly
        assert!(is_some(ns.resolve(GLOBAL, b"::tcl::mathfunc::sin")));
        // `sin` is NOT visible unqualified from global
        assert!(!is_some(ns.resolve(GLOBAL, b"sin")));
        // an absolute reference to a global command works
        assert!(is_some(ns.resolve(mf, b"::set")));
        assert_eq!(ns.qualified_name(mf), b"::tcl::mathfunc");
    }

    #[test]
    fn namespace_path_fallback() {
        let mut ns = Namespaces::new();
        ns.register(b"::tcl::mathop::+", cmd());
        let mathop = ns.find_namespace(GLOBAL, b"::tcl::mathop").unwrap();
        let foo = ns.ensure_namespace(GLOBAL, b"::foo");
        // bare `+` is not resolvable from ::foo …
        assert!(!is_some(ns.resolve(foo, b"+")));
        // … until ::tcl::mathop is on ::foo's namespace path.
        ns.set_path(foo, vec![mathop]);
        assert!(is_some(ns.resolve(foo, b"+")));
    }
}
