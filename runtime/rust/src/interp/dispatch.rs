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

//! Invoking a command: the dispatch boundary, traces, missing commands, bound invocation, procedure calls, tailcalls, ensembles and aliases.

use super::commands::Command;
use super::native_procedure_resources::NativeCallableProcedure;
use super::{Interp, NativeProcEntry, ProcDef};
use crate::namespace::{CommandBinding, NsId};
use crate::obj;
use crate::obj::TclObj;
use std::rc::Rc;

mod bound_invocation;
mod command_dispatch;
mod ensembles;
mod procedure_calls;
mod tailcalls;
mod traces;

#[cfg(test)]
pub(super) use ensembles::EnsembleUnknown;

/// The kind of body-frame a proc-style caller appends to the error trace when
/// its body throws (`MakeProcError` / `MakeLambdaError`, `tclProc.c`). Both
/// truncate the name to 60 bytes (`...` on overflow) and cite the body-relative
/// `error_line` left by the innermost logged command.
pub(crate) enum ProcFrame<'a> {
    /// `(procedure "NAME" line N)` — a named `proc`. `NAME` is the invoked name.
    Proc(&'a [u8]),
    /// `(lambda term "LAMBDA" line N)` — an `apply` lambda. `LAMBDA` is the whole
    /// lambda-expression string (`{params body ?ns?}`, i.e. `argv[1]`).
    Lambda(&'a [u8]),
    /// Jim records its original invocation without a lambda-expression display.
    JimLambda,
    /// A TclOO method body (`CommonMethErrorHandler`, `tclOOMethod.c`):
    /// `(KIND "OWNER" method "NAME" line N)`, or `(KIND "OWNER" constructor|
    /// destructor line N)`. `kind` is `object`/`class` per the declaring entity,
    /// `owner` is that entity's name, and `what` selects the method/ctor/dtor.
    Method {
        kind: &'a [u8],
        owner: &'a [u8],
        what: MethodFrameWhat<'a>,
    },
}

/// What a TclOO method-body error frame names: a method (`method "NAME"`) or a
/// constructor/destructor (a bare keyword).
pub(crate) enum MethodFrameWhat<'a> {
    Named(&'a [u8]),
    Constructor,
    Destructor,
}

/// What a proc/lambda call contributes to the diagnostic stacks: the errorInfo
/// frame (PC-4) plus the `info frame` proc FQN and defining-source (PC-5).
pub(crate) struct CallMeta<'a> {
    /// Borrowed from the active dispatcher; the procedure frame owns no extra
    /// object references merely to record its original invocation.
    pub original_argv: Option<&'a [*mut TclObj]>,
    /// The errorInfo `(procedure/lambda ...)` frame.
    pub err: ProcFrame<'a>,
    /// The proc's FQN — the `info frame` `proc` key (`None` for a lambda).
    pub fqn: Option<&'a [u8]>,
    /// The file the body was defined in (`source`d) — makes its `info frame`
    /// `type source` with this `file`.
    pub source: Option<Rc<[u8]>>,
    /// The body's `info frame` line base (file-absolute for a source-defined
    /// proc, 0 otherwise).
    pub body_line_base: u32,
    /// `(local, target)` instance-variable links to pre-install into the call
    /// frame: the method's declared variables, where `target` is the namespace
    /// storage name (== `local` for public vars, a mangled name for TIP 500
    /// private vars). Empty for procs/lambdas.
    pub link_vars: &'a [(Vec<u8>, Vec<u8>)],
    /// Current declaring-provider resolver, issued only by the reached OO body.
    pub oo_variable_resolver: Option<crate::cmd_oo::native_variables::NativeOoVariableResolver>,
    /// Return a body-level `break`/`continue` as the raw `Code` instead of the
    /// `invoked "break" outside of a loop` error. Set for TIP 558 property
    /// accessor methods, whose `configure` caller maps the loop codes to its
    /// own diagnostics.
    pub keep_loop_codes: bool,
    /// Run the body at the *current* call-frame level rather than pushing a new
    /// one (it still gets its own locals). Set for a TclOO method reached via
    /// `next`: every method in a single call chain shares the level of the
    /// original invocation, so `info level` / `upvar` / `uplevel` see through
    /// the chain to the original caller (C's call-chain execution).
    pub same_level: bool,
    /// The command prefix to use in a `wrong # args` message instead of
    /// `usage_called` (which still names the `info level` words). For a TclOO
    /// method this is the invoking `obj method` (or a forward's rewritten
    /// original invocation); `None` keeps `usage_called`.
    pub usage_prefix: Option<Vec<u8>>,
    /// The exact words to record for `info level N` of this frame, when they
    /// differ from the default `usage_called` + supplied args. Set for a TclOO
    /// constructor (the `create`/`new` invocation, e.g. `oo::object create foo`)
    /// so `info level 0` reflects the instantiation, not `<constructor>`.
    pub level_words: Option<Vec<Vec<u8>>>,
    /// List-quote the command name in a `wrong # args` message (Bug 942757) — a
    /// genuine single-word proc name (`a b  c` → `{a b  c}`, `` → `{}`). Off for
    /// `apply`/TclOO whose `usage_called`/`usage_prefix` is a pre-joined
    /// multi-word string (`apply lambdaExpr`, `obj method`) that must stay raw.
    pub quote_name: bool,
    /// The compiled body to run instead of the source one, from
    /// [`ProcDef::native`]. `None` for `apply` and every TclOO method: only a
    /// ordinary command can carry that entry; commandless methods and lambdas use
    /// their genuine original Proc body through `c_procedure`.
    pub native: Option<NativeProcEntry>,
    /// Persistent native procedure slots shared by the definition and activations.
    pub statics: Option<Rc<crate::frame::StaticVariables>>,
    pub c_procedure: Option<&'a Rc<ProcDef>>,
    /// Actual TclOO ProcedureMethod clientData, retained only after compilation.
    pub c_method_client_data: Option<&'a NativeCallableProcedure>,
    pub jim_parameters: Option<*mut TclObj>,
    pub jim_body: Option<*mut TclObj>,
    pub jim_namespace: Option<&'a Rc<obj::Owned>>,
}

impl<'a> CallMeta<'a> {
    /// Borrow genuine C or Jim invocation objects without reading argument Strings.
    /// Other frame formats keep their independently selected word transport.
    pub(crate) fn original_frame_arguments(
        dialect: tcl_registry::InvocationDialect,
        original_argv: Option<&'a [*mut TclObj]>,
    ) -> Option<&'a [*mut TclObj]> {
        dialect.native_string_protocol().and(original_argv)
    }
}

/// A selected miss retains its own lookup context instead of restarting lookup
/// in the variable frame restored after a tailcall or alias invocation.
enum CommandDispatchSelection {
    Unselected,
    LookupAt(NsId, tcl_registry::command_lookup::CommandLookupOrigin),
    Bound(Vec<u8>, CommandBinding),
    Missing {
        lookup: NsId,
        caller: NsId,
        origin: tcl_registry::command_lookup::CommandLookupOrigin,
    },
}

/// Prepared original unknown-prefix words and the selected handler. Keeping
/// preparation off the recursive invocation stack leaves only the actual
/// callback ownership and namespace restoration live during re-entry.
struct PreparedMissingCommand {
    owned: Vec<obj::Owned>,
    pub(super) words: Vec<*mut TclObj>,
    pub(super) selected: Option<(Command, Option<u64>)>,
    pub(super) fqn: Option<Vec<u8>>,
}

impl From<Option<(Vec<u8>, CommandBinding)>> for CommandDispatchSelection {
    fn from(binding: Option<(Vec<u8>, CommandBinding)>) -> Self {
        binding.map_or(Self::Unselected, |(name, binding)| {
            Self::Bound(name, binding)
        })
    }
}

/// Maximum nested alias-redirect depth — the native-stack bound on
/// alias-of-alias chains, matching [`NATIVE_EVAL_DEPTH_LIMIT`]'s budget. Real
/// chains are a hop or two; only a cycle the definition gate somehow missed
/// gets anywhere near it.
pub(super) const MAX_ALIAS_DISPATCH_DEPTH: u32 = 128;

/// Maximum nested cross-interp call depth (the native-stack bound for the
/// re-entrant parent⇄child recursion the Safe Base needs). Generous enough for
/// safe-base setup (a handful of hops) while still catching runaway recursion.
const MAX_CROSS_INTERP_DEPTH: u32 = 80;

impl Interp {
    /// Invoke a Tcl 8.4 fixed-table `expr` math function selected by the
    /// registry. This bypasses the command table deliberately: TIP 232 had not
    /// introduced `::tcl::mathfunc::*` command wrappers yet, so a similarly
    /// named proc or alias cannot replace the C function-table entry.
    #[cfg(have_tommath)]
    pub(crate) fn eval_fixed_math_call(
        &mut self,
        spec: &'static tcl_registry::CommandSpec,
        args: &[*mut TclObj],
    ) -> Code {
        let name = new_string(spec.name.as_bytes());
        let mut argv = Vec::with_capacity(args.len() + 1);
        // SAFETY: the fresh name and each live argument gain the ownership the
        // temporary argv carries until `release_all` below.
        unsafe { obj::incr_ref_count(name) };
        argv.push(name);
        for &arg in args {
            unsafe { obj::incr_ref_count(arg) };
            argv.push(arg);
        }
        let code = crate::cmd_mathfunc::mathfunc_identity(self, &argv, spec.name.as_bytes());
        release_all(&argv);
        code
    }

    /// Invoke `::tcl::mathfunc::<fname>` (resolved **absolutely**, so it works
    /// from any current namespace) with `args` as `objv` — the hook `expr`'s
    /// function-call path uses so a user-defined / overridden / renamed
    /// `::tcl::mathfunc::NAME` wins (the A3 contract). Leaves the result in the
    /// interp result; a missing command reports C's `invalid command name
    /// "tcl::mathfunc::NAME"` (no leading `::`, matching tclsh).
    #[cfg(have_tommath)]
    pub(crate) fn eval_math_call(&mut self, fname: &[u8], args: &[*mut TclObj]) -> Code {
        // C's expr emits the *relative* name `tcl::mathfunc::NAME`, resolved
        // from the CURRENT namespace by the ordinary command rule — so
        // `::ns::tcl::mathfunc::NAME` shadows the global function inside
        // `::ns` (tclsh 8.6.16/9.0.4-pinned; see the mathfunc rows in the
        // shared conformance vector table). The global `::tcl::mathfunc`
        // is simply the final fall-through base.
        let mut rel = b"tcl::mathfunc::".to_vec();
        rel.extend_from_slice(fname);
        let cmd = self.resolve_dispatchable_with_generation(self.current_ns.get(), &rel);
        let Some((cmd, generation)) = cmd else {
            let mut m = b"invalid command name \"tcl::mathfunc::".to_vec();
            m.extend_from_slice(fname);
            m.push(b'"');
            let error_code = error_code_list(&[b"TCL", b"LOOKUP", b"COMMAND", &rel]);
            return self.error_with_code(&m, &error_code);
        };
        // Build [name, args…], each owned (+1), and invoke the resolved command
        // directly (already resolved above; no second resolution).
        let mut argv: Vec<*mut TclObj> = Vec::with_capacity(args.len() + 1);
        let name_obj = new_string(&rel);
        // SAFETY: name_obj is fresh; args are live; take the owning +1 the argv holds.
        unsafe { obj::incr_ref_count(name_obj) };
        argv.push(name_obj);
        for &a in args {
            unsafe { obj::incr_ref_count(a) };
            argv.push(a);
        }
        let code = self.invoke_bound(cmd, generation, &argv);
        release_all(&argv);
        code
    }
}
