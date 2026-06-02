"""Flow-sensitive command-binding lattice.

A single, principled source of truth for "what does *this command name* resolve
to at this program point" — the property the optimiser must respect before it
constant-folds a builtin command substitution, rewrites an ``incr`` idiom, or
inlines a static proc call, and the property new diagnostics use to reason about
``rename`` / ``interp alias`` / proc redefinition.

The Tcl command table is mutable interpreter state: ``rename``, ``proc`` (re)defs
and ``interp alias`` rebind names *as the script runs*.  A flat whole-unit table
cannot tell ``call a`` apart before vs. after ``rename a b`` — so this is a
forward data-flow lattice over the CFG (a :class:`Binding` per command name,
joined to ``UNKNOWN`` when predecessors disagree), exactly like
:mod:`compiler.var_observability` does for variable aliasing.  A ``rename`` only
perturbs calls that *follow* it; code before it keeps the original binding.

``unknown`` semantics
---------------------
A name that has been renamed/deleted away (or was never defined) is **not** an
error and **not** dead code: Tcl dispatches the call to the ``unknown`` handler
(auto-load, ``package require``, ``namespace unknown`` …).  Such a name is
therefore modelled as :data:`BindingKind.OPAQUE` — *opaque*, never foldable, but
not assertable-as-broken.  Redefining ``unknown`` itself, ``namespace unknown``,
or any *dynamic* ``rename $x …`` widens resolution for **every** unbound name, so
those collapse the whole state to a wildcard ``UNKNOWN`` from that point on.
"""

from __future__ import annotations

from collections.abc import Iterable
from dataclasses import dataclass
from enum import Enum, auto

from shared.alias import detect_interp_alias
from shared.naming import normalise_qualified_name as _NQN

from .cfg import CFGBranch, CFGFunction, CFGGoto
from .ir import (
    IRBarrier,
    IRCall,
    IRCatch,
    IRFor,
    IRForeach,
    IRIf,
    IRModule,
    IRScript,
    IRStatement,
    IRSwitch,
    IRTry,
    IRWhile,
)
from .registry import REGISTRY


class BindingKind(Enum):
    """What a command name resolves to at a program point.

    Ordered as a lattice of height 3: ``BOTTOM`` (unreached) < any concrete
    binding < ``UNKNOWN`` (⊤).  Two *different* concrete bindings join to
    ``UNKNOWN``; a binding joined with itself is unchanged.
    """

    BOTTOM = auto()  # ⊥ — block not yet reached by the fixpoint
    BUILTIN = auto()  # the original core/registry command, unperturbed
    PROC = auto()  # a user procedure (``target`` = its canonical qname)
    ALIAS = auto()  # an ``interp alias`` (``target`` = the alias target name)
    OPAQUE = auto()  # renamed/deleted-away or never-defined → dispatches to
    #                 ``unknown`` / auto-load: opaque, never foldable, NOT an error
    UNKNOWN = auto()  # ⊤ — conflicting bindings at a merge, or dynamic mutation


@dataclass(frozen=True, slots=True)
class Binding:
    """A command name's resolution at a point: a :class:`BindingKind` + target."""

    kind: BindingKind
    target: str | None = None

    @property
    def is_original_builtin(self) -> bool:
        """True when the name still denotes its original core command."""
        return self.kind is BindingKind.BUILTIN

    @property
    def is_foldable_proc(self) -> bool:
        """True when the name denotes a single, known user proc."""
        return self.kind is BindingKind.PROC and self.target is not None


_BOTTOM = Binding(BindingKind.BOTTOM)
_TOP = Binding(BindingKind.UNKNOWN)
_OPAQUE = Binding(BindingKind.OPAQUE)
_BUILTIN = Binding(BindingKind.BUILTIN)

# A sparse per-name map; an absent name takes its *default* binding (a pure
# function of the name — builtin if the registry knows it, else opaque).  The
# wildcard key marks "every name is ⊤ from here" after a dynamic mutation; it
# can never collide with a real name because normalised names start with "::".
_State = dict[str, Binding]
_WILDCARD = "*"


def _default_binding(qname: str) -> Binding:
    """The unperturbed binding of *qname* before any rename/proc/alias event.

    A globally-scoped bare name the registry knows is its ``BUILTIN``; anything
    else is ``OPAQUE`` (undefined → resolved via ``unknown`` / auto-load until a
    ``proc`` / ``interp alias`` event binds it).
    """
    bare = qname[2:] if qname.startswith("::") else qname
    # Only an unqualified global name can be a core builtin (``::string`` →
    # ``string``); a namespaced tail like ``::ns::foo`` is never a builtin.
    if "::" not in bare and REGISTRY.get_any(bare) is not None:
        return _BUILTIN
    return _OPAQUE


def _binding_in(state: _State, qname: str) -> Binding:
    """Resolve *qname*'s binding within *state*, honouring wildcard + default."""
    if _WILDCARD in state:
        return _TOP
    return state.get(qname, _default_binding(qname))


def _join_binding(a: Binding, b: Binding) -> Binding:
    """Lattice join: ⊥ is identity, equal stays, anything else rises to ⊤."""
    if a.kind is BindingKind.BOTTOM:
        return b
    if b.kind is BindingKind.BOTTOM:
        return a
    if a == b:
        return a
    return _TOP


def _is_dynamic_word(word: str) -> bool:
    """True when *word* carries a variable / command substitution.

    A command-name argument that contains ``$`` or ``[`` *anywhere* — not just
    at offset 0 — is not a static name: e.g. ``rename ::$c mystr`` lowers the old
    name to ``::${c}``, and ``rename foo bar[x]`` makes the new name dynamic.
    Such a mutation can rebind an unknown command, so it must collapse the state
    to the wildcard ⊤ rather than be mistaken for a literal name.
    """
    return "$" in word or "[" in word


def _proc_qname_of(args: list[str]) -> str | None:
    """Canonical qname defined by a ``proc NAME params body`` call, or None."""
    if not args or not args[0]:
        return None
    name = args[0]
    if _is_dynamic_word(name):
        return None  # dynamic proc name — handled as a wildcard mutation
    return _NQN(name)


def _stmt_gen(stmt: IRStatement, state: _State) -> None:
    """Apply *stmt*'s command-table mutation to *state* in place.

    Recognises the three static rebinding forms — ``proc`` (definition /
    redefinition), ``rename`` (redirect or deletion) and ``interp alias`` — plus
    their *dynamic* variants, which collapse the state to a wildcard ⊤.
    """
    if not isinstance(stmt, (IRCall, IRBarrier)):
        return
    if _WILDCARD in state:
        return  # already maximally conservative
    cmd = stmt.canonical_command
    args = list(stmt.args)

    if cmd == "::proc":
        qname = _proc_qname_of(args)
        if qname is None:
            # ``proc $x …`` defines an unknown name — be conservative.
            state[_WILDCARD] = _TOP
            return
        # The name is now bound to this proc (whether it shadowed a builtin or a
        # prior proc).  We always record PROC: a *re*definition still leaves the
        # name a proc — the optimiser refuses to fold redefined procs via the
        # separate ``redefined_procedures`` gate, and a proc that *shadows a
        # builtin* is caught by the builtin-trust overlay (the name is no longer
        # its default BUILTIN, so builtin folding of it is disabled unit-wide).
        state[qname] = Binding(BindingKind.PROC, qname)
        return

    if cmd == "::rename":
        if len(args) != 2 or _is_dynamic_word(args[0]) or _is_dynamic_word(args[1]):
            # ``rename $old …`` / ``rename … [x]`` can touch any command.
            state[_WILDCARD] = _TOP
            return
        old, new = _NQN(args[0]), (args[1] and _NQN(args[1]))
        moved = _binding_in(state, old)
        # The old name is now unbound → falls through to ``unknown``/auto-load.
        state[old] = _OPAQUE
        if new:
            # The new name inherits whatever the old name denoted.
            state[new] = moved
        return

    if cmd == "::interp":
        detected = detect_interp_alias(stmt.command, args)
        if detected is not None:
            alias_name, target_cmd, _prepended = detected
            if _is_dynamic_word(alias_name) or _is_dynamic_word(target_cmd):
                state[_WILDCARD] = _TOP
                return
            state[_NQN(alias_name)] = Binding(BindingKind.ALIAS, _NQN(target_cmd))
            return
        # ``interp alias`` with a non-current target path, or a dynamic form we
        # could not destructure — only the alias subcommand mutates bindings.
        if args and args[0] == "alias":
            state[_WILDCARD] = _TOP
        return


def _merge_preds(pred_exits: list[_State]) -> _State:
    """Join predecessor exit states into a block-entry state.

    Sparsity carries *two distinct* notions of "absent" that must not be
    conflated: a name absent from a finished predecessor exit takes its
    **default** binding, whereas a name not yet contributed to the running
    accumulator is **⊥** (identity for join).  So the merge is computed
    per-name across *all* predecessors at once, seeding the accumulator at ⊥.
    A single predecessor whose state is the wildcard ⊤ forces every name to ⊤,
    i.e. the whole merge is the wildcard state.
    """
    if not pred_exits:
        return {}
    if any(_WILDCARD in pe for pe in pred_exits):
        return {_WILDCARD: _TOP}

    relevant: set[str] = set()
    for pe in pred_exits:
        relevant.update(pe)
    entry: _State = {}
    for name in relevant:
        acc = _BOTTOM
        for pe in pred_exits:
            acc = _join_binding(acc, pe.get(name, _default_binding(name)))
        if acc != _default_binding(name):
            entry[name] = acc
    return entry


def _successors(block) -> tuple[str, ...]:
    term = block.terminator
    if isinstance(term, CFGGoto):
        return (term.target,)
    if isinstance(term, CFGBranch):
        return (term.true_target, term.false_target)
    return ()


@dataclass(frozen=True, slots=True)
class CommandBinding:
    """Result of the command-binding analysis for one function/script.

    ``block_entry`` holds the lattice state at each block's entry; point-wise
    queries replay the gen of the statements before the queried index.
    """

    block_entry: dict[str, _State]
    _ordered_blocks: tuple[str, ...]
    _block_stmts: dict[str, tuple[IRStatement, ...]]

    def state_at(self, block: str, stmt_idx: int) -> _State:
        """Lattice state in force when ``block``'s statement *stmt_idx* runs."""
        state = dict(self.block_entry.get(block, {}))
        stmts = self._block_stmts.get(block, ())
        for i in range(min(stmt_idx, len(stmts))):
            _stmt_gen(stmts[i], state)
        return state

    def binding_at(self, block: str, stmt_idx: int, command_name: str) -> Binding:
        """The binding of *command_name* when ``block::stmt_idx`` executes."""
        return _binding_in(self.state_at(block, stmt_idx), _NQN(command_name))

    def is_original_builtin_at(self, block: str, stmt_idx: int, command_name: str) -> bool:
        """True when *command_name* still denotes its core builtin at this point."""
        return self.binding_at(block, stmt_idx, command_name).is_original_builtin

    def rebound_names(self) -> frozenset[str]:
        """Every command name perturbed from its default *anywhere* in the body.

        The flow-insensitive view: the union over all points of names whose
        binding ever differs from its default (renamed, redefined, aliased,
        deleted) — including names rebound transiently and later restored.
        """
        names: set[str] = set()
        for block in self._ordered_blocks:
            state = dict(self.block_entry.get(block, {}))
            for name, binding in state.items():
                if name == _WILDCARD or binding != _default_binding(name):
                    names.add(name)
            for stmt in self._block_stmts.get(block, ()):
                _stmt_gen(stmt, state)
                for name, binding in state.items():
                    if name == _WILDCARD or binding != _default_binding(name):
                        names.add(name)
        return frozenset(names)

    def has_wildcard(self) -> bool:
        """True when some path performs a *dynamic* command-table mutation."""
        for block in self._ordered_blocks:
            state = dict(self.block_entry.get(block, {}))
            if _WILDCARD in state:
                return True
            for stmt in self._block_stmts.get(block, ()):
                _stmt_gen(stmt, state)
                if _WILDCARD in state:
                    return True
        return False


def analyse_command_binding(
    cfg: CFGFunction,
    *,
    initial: dict[str, Binding] | None = None,
) -> CommandBinding:
    """Compute the flow-sensitive command-binding lattice for *cfg*.

    *initial* seeds the entry block's state — the command bindings already in
    force when this function begins.  The top-level analysis seeds it with every
    module procedure (``{qname: PROC(qname)}``) so a proc defined inside a
    ``namespace eval`` block (whose ``proc`` statement the top-level CFG never
    sees with its full qname) is still known to be a proc, while top-level
    ``rename`` / redefinition events still perturb it flow-sensitively.
    """
    blocks = cfg.blocks
    block_stmts = {name: blk.statements for name, blk in blocks.items()}
    seed: _State = dict(initial or {})

    preds: dict[str, list[str]] = {name: [] for name in blocks}
    for name, blk in blocks.items():
        for succ in _successors(blk):
            if succ in preds:
                preds[succ].append(name)
    for src, handler in cfg.exception_edges:
        if handler in preds and src in blocks:
            preds[handler].append(src)

    order = cfg.reverse_postorder()

    block_entry: dict[str, _State] = {name: {} for name in blocks}
    block_exit: dict[str, _State] = {name: {} for name in blocks}

    # Monotonic forward fixpoint: the per-name lattice has height 3 and the join
    # only ever rises (toward ⊤), so RPO iteration to a fixpoint terminates.
    changed = True
    while changed:
        changed = False
        for name in order:
            pred_exits = [block_exit[p] for p in preds.get(name, ())]
            if name == cfg.entry:
                pred_exits.append(seed)
            entry = _merge_preds(pred_exits)
            block_entry[name] = entry
            exit_state = dict(entry)
            for stmt in block_stmts.get(name, ()):
                _stmt_gen(stmt, exit_state)
            if exit_state != block_exit[name]:
                block_exit[name] = exit_state
                changed = True

    return CommandBinding(
        block_entry=block_entry,
        _ordered_blocks=tuple(order),
        _block_stmts=block_stmts,
    )


@dataclass(frozen=True, slots=True)
class ModuleCommandMutations:
    """Conservative, flow-insensitive summary of rebindings *inside proc bodies*.

    A ``rename`` / ``proc`` redef / ``interp alias`` buried in a proc body only
    takes effect when that proc is *called*, and the call order across procs is
    not statically known.  Rather than a full interprocedural call-effect
    fixpoint, v1 takes the sound over-approximation: any command name a proc body
    may rebind is treated as untrusted *everywhere*.  Top-level rebindings stay
    precise via the flow-sensitive :class:`CommandBinding` lattice.

    * ``names`` — canonical command names some proc/method body may rebind.
    * ``dynamic`` — a proc/method body performs a dynamic ``rename``/alias/proc
      (target not statically known), or redefines ``unknown`` / sets
      ``namespace unknown`` → resolution of *any* unbound name is opaque.
    """

    names: frozenset[str]
    dynamic: bool

    def trusts(self, command_name: str) -> bool:
        """True when *command_name* is not clobbered by any proc-body mutation."""
        if self.dynamic:
            return False
        return _NQN(command_name) not in self.names


def _iter_calls(script: IRScript | None) -> Iterable[IRStatement]:
    """Yield every ``IRCall`` / ``IRBarrier`` in *script*, recursing into the
    nested bodies of structured nodes (if / for / while / foreach / catch /
    try / switch) so a rebinding buried in control flow is not missed."""
    if script is None:
        return
    for stmt in script.statements:
        if isinstance(stmt, (IRCall, IRBarrier)):
            yield stmt
        elif isinstance(stmt, IRIf):
            for clause in stmt.clauses:
                yield from _iter_calls(clause.body)
            yield from _iter_calls(stmt.else_body)
        elif isinstance(stmt, IRFor):
            yield from _iter_calls(stmt.init)
            yield from _iter_calls(stmt.next)
            yield from _iter_calls(stmt.body)
        elif isinstance(stmt, IRWhile):
            yield from _iter_calls(stmt.body)
        elif isinstance(stmt, IRForeach):
            yield from _iter_calls(stmt.body)
        elif isinstance(stmt, IRCatch):
            yield from _iter_calls(stmt.body)
        elif isinstance(stmt, IRTry):
            yield from _iter_calls(stmt.body)
            for handler in stmt.handlers:
                yield from _iter_calls(handler.body)
            yield from _iter_calls(stmt.finally_body)
        elif isinstance(stmt, IRSwitch):
            for arm in stmt.arms:
                yield from _iter_calls(arm.body)
            yield from _iter_calls(stmt.default_body)


def scan_module_command_mutations(module: IRModule) -> ModuleCommandMutations:
    """Summarise command-table mutations across the whole module.

    A CFG-free recursive IR walk over the top-level script *and* every
    proc/method body (so it can run *before* the per-function CFGs are built
    during compilation-unit assembly).  Flow-insensitive by construction — the
    builtin-trust verdict only needs the *set* of names the unit may rebind, so
    applying the gen of every nested ``IRCall`` into one accumulating state and
    collecting the perturbed names (and any wildcard) is sufficient and sound.

    For top-level *flow-sensitive* reasoning (``call a; rename a b; call b``) use
    :func:`analyse_command_binding` directly; this whole-module union is the
    conservative input to the builtin-fold / builtin-dispatch gate.

    Only **core builtins that were tampered with** are reported — a name whose
    *default* binding is ``BUILTIN`` and whose observed binding diverges from it
    (renamed away → ``OPAQUE``; shadowed/redefined → ``PROC`` / ``UNKNOWN``).  A
    freshly-defined user proc (default ``OPAQUE`` → ``PROC``) is deliberately
    *excluded*: it doesn't untrust any builtin, and including it would wrongly
    make every user-proc call route through the interpreter in codegen.
    """
    names: set[str] = set()
    dynamic = False

    def _collect(state: _State) -> None:
        nonlocal dynamic
        if _WILDCARD in state:
            dynamic = True
        for name, binding in state.items():
            if name == _WILDCARD:
                continue
            default = _default_binding(name)
            if binding != default and default.kind is BindingKind.BUILTIN:
                names.add(name)

    def visit(script: IRScript | None) -> None:
        state: _State = {}
        # Collect after *each* mutation, not just the final state: a builtin
        # renamed away and later restored (``rename string ms; …; rename ms
        # string``) ends back at its default but was tampered with mid-body, so
        # calls in that window must not be trusted/folded.  Mirrors
        # ``CommandBinding.rebound_names``.
        for stmt in _iter_calls(script):
            _stmt_gen(stmt, state)
            _collect(state)

    visit(module.top_level)
    for proc in module.procedures.values():
        visit(proc.body)
    for method in module.methods.values():
        visit(method.body)

    return ModuleCommandMutations(names=frozenset(names), dynamic=dynamic)


def scan_command_mutations_in_bodies(
    cfgs: Iterable[CFGFunction],
) -> ModuleCommandMutations:
    """Summarise command-table mutations across a set of proc/method body CFGs.

    Driven off each body's *CFG* (not its raw statement stream) so renames buried
    in control flow inside the body are caught.  Reuses the same lattice as the
    flow-sensitive top-level analysis — one grammar, one gen function.

    As with :func:`scan_module_command_mutations`, only *tampered-with core
    builtins* are reported (a freshly-defined user proc is excluded).
    """
    names: set[str] = set()
    dynamic = False
    for cfg in cfgs:
        cb = analyse_command_binding(cfg)
        if cb.has_wildcard():
            dynamic = True
        for name in cb.rebound_names():
            if name != _WILDCARD and _default_binding(name).kind is BindingKind.BUILTIN:
                names.add(name)
    return ModuleCommandMutations(names=frozenset(names), dynamic=dynamic)
