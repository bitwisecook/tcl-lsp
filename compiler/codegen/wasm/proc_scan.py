"""Dynamic proc-invalidation scan for WASM codegen.

When a module uses ``rename`` / ``interp hide`` / ``interp expose`` (or
the child-interp primitives ``interp create`` / ``eval`` / ``delete``),
the compile-time direct-call specialisation is unsound for the affected
procs.  :func:`wasm_codegen_module` consults
:func:`_collect_dynamically_modified_procs` to downgrade those calls to
the runtime ``tcl_eval`` / ``proc_lookup`` path.
"""

from __future__ import annotations

from ...ir import (
    IRBlock,
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


def _proc_name_variants(name: str, context_ns: str = "::") -> tuple[str, ...]:
    """Candidate ``proc_index`` keys for a user-supplied name in a
    given namespace context.

    The user's spelling of a command name on a ``rename`` / ``interp
    hide`` line can be unqualified (``foo``) or fully qualified
    (``::foo``, ``::ns::foo``).  The compile-time ``proc_index`` keys
    are always fully qualified, but resolution of an unqualified
    token depends on the enclosing namespace: inside
    ``namespace eval ::ns { rename foo bar }`` the target is
    ``::ns::foo``, not ``::foo``.

    We produce:

    * ``name`` verbatim (for users who wrote the qualified form
      directly, and as a defensive pass-through).
    * ``::<name>`` when the name is unqualified (root-scope fallback).
    * ``<context_ns>::<name>`` when the name is unqualified and the
      context is not root (``namespace eval`` body, or the
      enclosing proc's home ns).

    Brace-wrapping (``{foo}``) survives as-is because the IR lowers
    brace-quoted words to their inner bytes.
    """
    name = name.strip()
    if not name:
        return ()
    if name.startswith("::"):
        return (name,)
    variants: list[str] = [name, f"::{name}"]
    if context_ns and context_ns != "::":
        # ``context_ns`` is expected to be FQN-shaped (``::ns`` /
        # ``::ns::sub``).  Normalise just in case a caller passed a
        # trailing ``::``.
        base = context_ns.rstrip(":")
        variants.append(f"{base}::{name}")
    return tuple(variants)


def _collect_dynamically_modified_procs(
    ir_module: IRModule,
) -> tuple[set[tuple[str, str]], bool]:
    """Return the set of ``(context_ns, name)`` pairs for procs that
    ``rename`` / ``interp hide`` / ``interp expose`` touches
    anywhere in ``ir_module``, plus a ``full_flush`` flag that's
    ``True`` when the module uses a command whose side-effects can't
    be tracked surgically (``interp create`` / ``interp eval`` /
    ``interp delete``).

    Used by :func:`wasm_codegen_module` to invalidate
    ``proc_index`` entries for those names, downgrading calls from
    a compile-time specialised ``call $::foo`` to the runtime
    ``tcl_eval`` / ``proc_lookup`` path so runtime hide / rename
    state is visible.  When ``full_flush`` is set, the caller drops
    *every* entry — the conservative shortcut documented in
    ``docs/design/runtime/child-interp.md`` §7 for the child-interp
    wave.

    The walker threads a ``context_ns`` through every script body:

    * ``IRBlock`` — from ``namespace eval`` — pushes its own
      ``namespace`` field.
    * Procedure / method bodies start in the proc's enclosing
      namespace (``::ns::foo`` runs with context ``::ns``).
    * All other compound nodes inherit the current context.

    The context is paired with every invalidation target so the
    caller can qualify unqualified names against the right
    namespace — unqualified ``rename foo bar`` inside
    ``namespace eval ::ns { … }`` must invalidate
    ``::ns::foo`` / ``::ns::bar``, not just ``::foo`` / ``::bar``.
    """
    affected: set[tuple[str, str]] = set()
    full_flush = [False]

    def _scan_statement(stmt: IRStatement, context_ns: str) -> None:
        match stmt:
            case IRCall(command=cmd, args=args):
                if cmd == "rename" and len(args) >= 1:
                    affected.add((context_ns, args[0]))
                    if len(args) >= 2:
                        affected.add((context_ns, args[1]))
                elif cmd == "interp" and args and args[0] in ("hide", "expose"):
                    # ``interp hide path cmd ?hiddenName?`` →
                    # args = ("hide", path, cmd, [hiddenName]).
                    # Both ``cmd`` and ``hiddenName`` / ``newName``
                    # name procs that may disappear or re-appear at
                    # runtime, so both are invalidated.
                    if len(args) >= 3:
                        affected.add((context_ns, args[2]))
                    if len(args) >= 4:
                        affected.add((context_ns, args[3]))
                elif (
                    cmd == "interp"
                    and args
                    and args[0]
                    in (
                        "create",
                        "eval",
                        "delete",
                    )
                ):
                    # Child-interp primitives: a child can define
                    # arbitrary procs whose names we can't enumerate
                    # statically, and ``interp delete`` can unlink
                    # procs we might be specialising.  Full flush is
                    # the conservative-but-correct shortcut — calls
                    # route through ``tcl_eval`` and observe the live
                    # registry.
                    full_flush[0] = True
            case IRIf(clauses=clauses, else_body=else_body):
                for clause in clauses:
                    _scan_script(clause.body, context_ns)
                if else_body is not None:
                    _scan_script(else_body, context_ns)
            case IRFor(init=init, body=body, next=next_s):
                _scan_script(init, context_ns)
                _scan_script(body, context_ns)
                _scan_script(next_s, context_ns)
            case IRWhile(body=body):
                _scan_script(body, context_ns)
            case IRForeach(body=body):
                _scan_script(body, context_ns)
            case IRSwitch(arms=arms, default_body=default_body):
                for arm in arms:
                    if arm.body is not None:
                        _scan_script(arm.body, context_ns)
                if default_body is not None:
                    _scan_script(default_body, context_ns)
            case IRCatch(body=body):
                _scan_script(body, context_ns)
            case IRTry(body=body, finally_body=finally_body):
                _scan_script(body, context_ns)
                if finally_body is not None:
                    _scan_script(finally_body, context_ns)
            case IRBlock(body=body, namespace=ns):
                # ``namespace eval ::ns { … }`` — the body runs with
                # the block's own namespace, not the enclosing one.
                _scan_script(body, ns if ns else context_ns)
            case _:
                pass

    def _scan_script(script: IRScript, context_ns: str) -> None:
        for stmt in script.statements:
            _scan_statement(stmt, context_ns)

    _scan_script(ir_module.top_level, "::")
    for qname, proc in ir_module.procedures.items():
        # Derive the enclosing ns from the proc's qualified name:
        # ``::ns::foo`` runs in ``::ns``; ``::foo`` runs in ``::``.
        parent_ns = qname.rsplit("::", 1)[0] or "::"
        _scan_script(proc.body, parent_ns)
    if ir_module.methods:
        for method in ir_module.methods.values():
            # Methods run in their class's namespace; use root as a
            # conservative default — the class name isn't a
            # ``namespace eval`` target, but qualified ``rename``
            # inside a method is so rare that the extra root probe
            # is harmless.
            _scan_script(method.body, "::")
    return affected, full_flush[0]
