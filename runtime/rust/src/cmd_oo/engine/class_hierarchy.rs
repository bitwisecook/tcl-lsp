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

//! Queries over the class hierarchy: precedence and C3 linearisation, property lists, and where a method is defined, exported or private.

use super::super::{MAX_MRO_DEPTH, MAX_MRO_VISITS};
use crate::interp::Interp;
use tcl_core_types::OoId;

impl Interp {
    /// The precedence (linearisation) of a *class* itself — its mixins, the
    /// class, then its superclasses — keep-last deduped, for `info class call`.
    pub(in super::super) fn class_precedence(&self, class: OoId) -> Vec<OoId> {
        let mut seq: Vec<OoId> = Vec::new();
        let mut path: Vec<OoId> = Vec::new();
        let mut budget = MAX_MRO_VISITS;
        self.linearize_class(class, &mut seq, &mut path, 0, &mut budget);
        let mut out: Vec<OoId> = Vec::with_capacity(seq.len());
        for (i, c) in seq.iter().enumerate() {
            if !seq[i + 1..].iter().any(|x| x == c) {
                out.push(*c);
            }
        }
        out
    }

    /// Depth-first precedence of a class into `seq` (with duplicates, resolved by
    /// the caller's keep-last dedup): the class's mixins (recursively), then the
    /// class itself, then its superclasses. `path` breaks cycles in a malformed
    /// hierarchy without suppressing the legitimate re-visits keep-last relies on.
    ///
    /// `depth` is this call's nesting level (0 at the root) and `budget` is the
    /// remaining total-visit allowance shared across the whole linearisation
    /// (both callers') — see [`MAX_MRO_DEPTH`]/[`MAX_MRO_VISITS`]. Past
    /// either cap, this call stops descending without recursing further:
    /// the already-linearised prefix in `seq` is kept as-is, the same graceful
    /// "just stop" degradation the `path` cycle guard above already applies to
    /// a malformed hierarchy, rather than overflowing the native stack or
    /// hanging on an adversarial diamond-mixin shape.
    pub(in super::super) fn linearize_class(
        &self,
        class: OoId,
        seq: &mut Vec<OoId>,
        path: &mut Vec<OoId>,
        depth: u32,
        budget: &mut u32,
    ) {
        if MAX_MRO_DEPTH.exceeded(depth) || *budget == 0 {
            return;
        }
        *budget -= 1;
        if path.contains(&class) {
            return;
        }
        path.push(class);
        let (mixins, supers) = {
            let oo = self.oo.borrow();
            match oo.classes.get(&class) {
                Some(c) => (c.mixins.clone(), c.supers.clone()),
                None => (Vec::new(), Vec::new()),
            }
        };
        for mx in &mixins {
            self.linearize_class(*mx, seq, path, depth + 1, budget);
        }
        seq.push(class);
        for s in &supers {
            self.linearize_class(*s, seq, path, depth + 1, budget);
        }
        path.pop();
    }

    /// Sorted (ASCII) unique property names for a class. With `all`, unions the
    /// class's readable/writable set over its mixins and superclasses (C's
    /// `FindClassProps`); otherwise just the class's own set (oo-1.1–1.6).
    pub(in super::super) fn class_property_list(
        &self,
        class: OoId,
        writable: bool,
        all: bool,
    ) -> Vec<Vec<u8>> {
        let mut acc: Vec<Vec<u8>> = Vec::new();
        if all {
            let mut seen: Vec<OoId> = Vec::new();
            let mut budget = MAX_MRO_VISITS;
            self.gather_class_props(class, writable, &mut acc, &mut seen, 0, &mut budget);
        } else if let Some(c) = self.oo.borrow().classes.get(&class) {
            let src = if writable {
                &c.writable_properties
            } else {
                &c.readable_properties
            };
            acc.extend(src.iter().cloned());
        }
        acc.sort();
        acc.dedup();
        acc
    }

    /// `depth`/`budget` — see [`linearize_class`](Self::linearize_class), which
    /// this mirrors. Unlike `linearize_class`'s `path`,
    /// `gather_class_props`'s `seen` is a *global* (never-popped) visited set,
    /// so a diamond is naturally visited once, not once per reaching path —
    /// `budget` is therefore more a defensive second layer than a load-bearing
    /// one here, added for consistency with the sibling walk rather than
    /// because this shape is independently known to blow up; `depth` is the
    /// one that matters, guarding a long mixin/superclass *chain* the same way
    /// `linearize_class`'s does.
    pub(in super::super) fn gather_class_props(
        &self,
        class: OoId,
        writable: bool,
        acc: &mut Vec<Vec<u8>>,
        seen: &mut Vec<OoId>,
        depth: u32,
        budget: &mut u32,
    ) {
        if MAX_MRO_DEPTH.exceeded(depth) || *budget == 0 {
            return;
        }
        *budget -= 1;
        if seen.contains(&class) {
            return;
        }
        seen.push(class);
        let (props, mixins, supers) = {
            let oo = self.oo.borrow();
            match oo.classes.get(&class) {
                Some(c) => {
                    let p = if writable {
                        c.writable_properties.clone()
                    } else {
                        c.readable_properties.clone()
                    };
                    (p, c.mixins.clone(), c.supers.clone())
                }
                None => return,
            }
        };
        for p in props {
            if !acc.contains(&p) {
                acc.push(p);
            }
        }
        for m in &mixins {
            self.gather_class_props(*m, writable, acc, seen, depth + 1, budget);
        }
        for s in &supers {
            self.gather_class_props(*s, writable, acc, seen, depth + 1, budget);
        }
    }

    /// Sorted unique property names for an object. With `all`, unions the
    /// object's own set with its full precedence (mixins + class chain);
    /// otherwise just the object's own set (oo-1.7–1.11).
    pub(in super::super) fn object_property_list(
        &self,
        obj: OoId,
        writable: bool,
        all: bool,
    ) -> Vec<Vec<u8>> {
        let mut acc: Vec<Vec<u8>> = Vec::new();
        let push = |src: &[Vec<u8>], acc: &mut Vec<Vec<u8>>| {
            for p in src {
                if !acc.contains(p) {
                    acc.push(p.clone());
                }
            }
        };
        if let Some(o) = self.oo.borrow().objects.get(&obj) {
            let src = if writable {
                &o.writable_properties
            } else {
                &o.readable_properties
            };
            push(src, &mut acc);
        }
        if all {
            for c in self.method_chain(obj) {
                if c == obj {
                    continue;
                }
                if let Some(cl) = self.oo.borrow().classes.get(&c) {
                    let src = if writable {
                        &cl.writable_properties
                    } else {
                        &cl.readable_properties
                    };
                    push(src, &mut acc);
                }
            }
        }
        acc.sort();
        acc.dedup();
        acc
    }

    /// Whether the provider `prov` (object FQN or class FQN) defines `method`
    /// (or a constructor, when `ctor`).
    /// Whether provider `prov` defines `method`. Resolution is positional: the
    /// object itself (`is_object`, the chain head) carries *per-object* methods
    /// (`objects[prov].methods`); the rest of the chain are classes carrying
    /// *instance* methods (`classes[prov].methods`). A class is registered in
    /// both maps (classes-as-objects), so this distinction must be by position,
    /// not by which map `prov` is in.
    pub(in super::super) fn oo_has_method(
        &self,
        prov: OoId,
        method: &[u8],
        is_object: bool,
    ) -> bool {
        if is_object {
            self.oo
                .borrow()
                .objects
                .get(&prov)
                .is_some_and(|o| o.methods.contains_key(method))
        } else {
            self.oo
                .borrow()
                .classes
                .get(&prov)
                .is_some_and(|c| c.methods.contains_key(method))
        }
    }

    /// Whether `obj` has a *user-defined* `unknown` method (one declared on the
    /// object or a class other than the `oo::object` root, whose built-in
    /// `unknown` is the default error). A no-method invocation (`$obj`) routes
    /// to a user handler but otherwise reports the `wrong # args` usage.
    pub(super) fn has_user_unknown(&self, obj: OoId) -> bool {
        self.method_chain(obj).iter().any(|p| {
            Some(*p) != self.oo.borrow().object_root
                && self.oo_has_method(*p, b"unknown", *p == obj)
        })
    }

    /// Whether the class `prov` defines a constructor.
    pub(super) fn class_has_ctor(&self, prov: OoId) -> bool {
        self.oo
            .borrow()
            .classes
            .get(&prov)
            .is_some_and(|c| c.constructor.is_some())
    }

    /// Whether `method` was explicitly `export`ed anywhere in `obj`'s method
    /// chain (the object itself or any class in its MRO) — promotes a default-
    /// unexported built-in (`eval`/`variable`/`varname`) to public.
    pub(super) fn method_exported(&self, obj: OoId, method: &[u8]) -> bool {
        self.method_chain(obj).iter().any(|p| {
            let oo = self.oo.borrow();
            if *p == obj {
                oo.objects
                    .get(p)
                    .is_some_and(|o| o.exported.contains(method))
            } else {
                oo.classes
                    .get(p)
                    .is_some_and(|c| c.exported.contains(method))
            }
        })
    }

    /// `(exported, unexported, private)` membership for `method` aggregated over
    /// `obj`'s whole method chain — used to resolve a built-in method's effective
    /// visibility for `info object methods -all`.
    /// The export state a class built-in takes from the first record naming
    /// it along the object's method chain: C's lookup reaches a body-less
    /// `export`/`unexport` record (such as `oo::abstract`'s `unexport new`)
    /// before the built-in. `None` where no record names it.
    pub(super) fn factory_export_record(&self, class: OoId, method: &[u8]) -> Option<bool> {
        self.method_chain_faceted(class)
            .into_iter()
            .find_map(|(provider, is_object)| {
                let oo = self.oo.borrow();
                let (exported, unexported) = if is_object {
                    oo.objects
                        .get(&provider)
                        .map(|o| (&o.exported, &o.unexported))?
                } else {
                    oo.classes
                        .get(&provider)
                        .map(|c| (&c.exported, &c.unexported))?
                };
                if unexported.contains(method) {
                    Some(false)
                } else {
                    exported.contains(method).then_some(true)
                }
            })
    }

    pub(in super::super) fn method_visibility_flags(
        &self,
        obj: OoId,
        method: &[u8],
    ) -> (bool, bool, bool) {
        let (mut exp, mut unexp, mut priv_) = (false, false, false);
        for p in self.method_chain(obj) {
            let oo = self.oo.borrow();
            let sets = if p == obj {
                oo.objects
                    .get(&p)
                    .map(|o| (&o.exported, &o.unexported, &o.private))
            } else {
                oo.classes
                    .get(&p)
                    .map(|c| (&c.exported, &c.unexported, &c.private))
            };
            if let Some((e, u, pr)) = sets {
                exp |= e.contains(method);
                unexp |= u.contains(method);
                priv_ |= pr.contains(method);
            }
        }
        (exp, unexp, priv_)
    }

    /// Whether `method` is a TIP 500 *private* method of `prov` (a subset of its
    /// unexported methods — scoped to `prov`'s own methods).
    pub(in super::super) fn method_is_private(
        &self,
        prov: OoId,
        method: &[u8],
        is_object: bool,
    ) -> bool {
        if is_object {
            self.oo
                .borrow()
                .objects
                .get(&prov)
                .is_some_and(|o| o.private.contains(method))
        } else {
            self.oo
                .borrow()
                .classes
                .get(&prov)
                .is_some_and(|c| c.private.contains(method))
        }
    }

    pub(in super::super) fn method_unexported(
        &self,
        prov: OoId,
        method: &[u8],
        is_object: bool,
    ) -> bool {
        if is_object {
            self.oo
                .borrow()
                .objects
                .get(&prov)
                .is_some_and(|o| o.unexported.contains(method))
        } else {
            self.oo
                .borrow()
                .classes
                .get(&prov)
                .is_some_and(|c| c.unexported.contains(method))
        }
    }

    /// The method-resolution order for `class`: a preorder walk of the class and
    /// its superclasses, each class appearing once.
    pub(in super::super) fn mro(&self, class: OoId) -> Vec<OoId> {
        let mut guard: Vec<OoId> = Vec::new();
        self.c3_linearize(class, &mut guard)
    }

    /// C3 linearisation of `class` (the class then its superclasses, each once,
    /// in C3 order — so a diamond `D(B C)`/`B(A)`/`C(A)` yields `[D B C A]`,
    /// deferring `A` until after both `B` and `C`). `guard` breaks cycles in a
    /// malformed hierarchy (falling back to a preorder remainder).
    fn c3_linearize(&self, class: OoId, guard: &mut Vec<OoId>) -> Vec<OoId> {
        if guard.contains(&class) {
            return vec![class];
        }
        guard.push(class);
        let supers = self
            .oo
            .borrow()
            .classes
            .get(&class)
            .map(|c| c.supers.clone())
            .unwrap_or_default();
        let result = if supers.is_empty() {
            vec![class]
        } else {
            let mut seqs: Vec<Vec<OoId>> = supers
                .iter()
                .map(|s| self.c3_linearize(*s, guard))
                .collect();
            seqs.push(supers.clone());
            let mut out = vec![class];
            out.extend(c3_merge(seqs));
            out
        };
        guard.pop();
        result
    }
}

/// The C3 merge: repeatedly take the head of the first sequence that does not
/// appear in the *tail* of any sequence, removing it from all. On an
/// inconsistent hierarchy (no valid head), append the remaining heads in order
/// (degraded, rather than looping).
fn c3_merge(mut seqs: Vec<Vec<OoId>>) -> Vec<OoId> {
    let mut out: Vec<OoId> = Vec::new();
    loop {
        seqs.retain(|s| !s.is_empty());
        if seqs.is_empty() {
            return out;
        }
        // A good head is the first sequence's head that is not in any tail.
        let head = seqs.iter().find_map(|s| {
            let h = &s[0];
            let in_tail = seqs
                .iter()
                .any(|t| t.len() > 1 && t[1..].iter().any(|x| x == h));
            if in_tail {
                None
            } else {
                Some(*h)
            }
        });
        let head = match head {
            Some(h) => h,
            // Inconsistent: break the deadlock with the first available head.
            None => seqs[0][0],
        };
        out.push(head);
        for s in &mut seqs {
            if s.first() == Some(&head) {
                s.remove(0);
            }
        }
    }
}
