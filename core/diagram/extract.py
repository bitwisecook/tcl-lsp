"""Extract structured flow data from an iRule for Mermaid diagram generation.

Walks the IR produced by :func:`lower_to_ir` and builds a JSON-serialisable
dict describing event handlers, decision points, and actions.  The output
is consumed by the ``/diagram`` VS Code chat command which forwards it to
the LLM together with the original source for Mermaid + explanation generation.
"""

# canonicalisation: audited #246

from __future__ import annotations

from ..commands.registry import REGISTRY
from ..commands.registry.namespace_registry import NAMESPACE_REGISTRY as EVENT_REGISTRY
from ..compiler.expr_ast import ExprNode, expr_text
from ..compiler.ir import (
    IRAssignConst,
    IRAssignExpr,
    IRAssignValue,
    IRBarrier,
    IRCall,
    IRCatch,
    IRFor,
    IRForeach,
    IRIf,
    IRProcedure,
    IRReturn,
    IRScript,
    IRStatement,
    IRSwitch,
    IRTry,
    IRWhile,
    when_event_name,
)
from ..compiler.lowering import lower_to_ir

_MAX_DEPTH = 8
_MAX_EVENTS = 12
_MAX_ARG_LEN = 60

# Helpers


def _truncate(text: str, limit: int = _MAX_ARG_LEN) -> str:
    if len(text) <= limit:
        return text
    return text[: limit - 3] + "..."


def _diagram_safe_operators(text: str) -> str:
    """Replace symbolic logical operators with words for Mermaid compatibility.

    Mermaid interprets ``&`` as an HTML entity start, so ``&&`` must become
    ``and``, ``||`` must become ``or``, and prefix ``!`` must become ``not``
    in node labels.
    """
    import re

    # Replace && with 'and' and || with 'or' when surrounded by spaces.
    text = re.sub(r"(?<=\s)&&(?=\s)", "and", text)
    text = re.sub(r"(?<=\s)\|\|(?=\s)", "or", text)
    # Replace prefix '!' (logical NOT) with 'not ' — but not '!=' (ne).
    text = re.sub(r"(^|\s)!(?!=)", r"\1not ", text)
    return text


def _condition_text(node: ExprNode) -> str:
    return _diagram_safe_operators(_truncate(expr_text(node), 80))


def _multiplicity(event_name: str) -> str:
    from core.commands.registry.namespace_data import event_multiplicity

    return event_multiplicity(event_name)


def _is_notable_assign(value: str) -> bool:
    """Return True if an assignment captures a command substitution worth showing."""
    return "[" in value


# IR walking


def _walk_statement(
    stmt: IRStatement,
    proc_names: frozenset[str],
    depth: int,
) -> dict | None:
    """Convert a single IR statement to a flow-node dict, or None to skip."""
    if depth > _MAX_DEPTH:
        return {"kind": "truncated", "label": "... (nested logic)"}

    match stmt:
        case IRSwitch(subject=subject, arms=arms, default_body=default_body):
            serialised_arms: list[dict] = []
            fallthrough_patterns: list[str] = []
            for arm in arms:
                if arm.fallthrough:
                    fallthrough_patterns.append(arm.pattern)
                    continue
                patterns = [*fallthrough_patterns, arm.pattern]
                fallthrough_patterns.clear()
                body = _walk_script(arm.body, proc_names, depth + 1) if arm.body else []
                serialised_arms.append(
                    {
                        "pattern": " | ".join(patterns) if len(patterns) > 1 else patterns[0],
                        "body": body,
                    }
                )
            if default_body is not None:
                body = _walk_script(default_body, proc_names, depth + 1)
                serialised_arms.append({"pattern": "default", "body": body})
            return {
                "kind": "switch",
                "subject": _truncate(subject, 80),
                "arms": serialised_arms,
            }

        case IRIf(clauses=clauses, else_body=else_body):
            branches: list[dict] = []
            for clause in clauses:
                body = _walk_script(clause.body, proc_names, depth + 1)
                branches.append(
                    {
                        "condition": _condition_text(clause.condition),
                        "body": body,
                    }
                )
            if else_body is not None:
                body = _walk_script(else_body, proc_names, depth + 1)
                branches.append({"condition": "else", "body": body})
            return {"kind": "if", "branches": branches}

        case IRFor(body=body):
            child = _walk_script(body, proc_names, depth + 1)
            return {"kind": "loop", "label": "for", "body": child}

        case IRWhile(condition=condition, body=body):
            child = _walk_script(body, proc_names, depth + 1)
            return {
                "kind": "loop",
                "label": f"while {_condition_text(condition)}",
                "body": child,
            }

        case IRForeach(iterators=iterators, body=body):
            vars_part = ", ".join(" ".join(vl) for vl, _ in iterators)
            child = _walk_script(body, proc_names, depth + 1)
            return {
                "kind": "loop",
                "label": f"foreach {_truncate(vars_part)}",
                "body": child,
            }

        case IRCall(args=args) as ir_call:
            # Match against the canonical form so qualified spellings,
            # ``interp alias`` aliases, and namespace-imported builtins
            # all hit; render the user's source spelling so the diagram
            # shows what they wrote.  See issue #246.
            canonical = ir_call.canonical_command
            display = ir_call.command
            # Skip the top-level 'when' calls — their bodies are in procedures.
            if canonical == "::when":
                return None
            # Procedure calls.
            if display in proc_names or canonical in proc_names:
                return {
                    "kind": "proc_call",
                    "label": f"call {display}",
                    "command": display,
                }
            # Notable action commands.
            if REGISTRY.is_diagram_action(canonical):
                arg_str = " ".join(_truncate(a) for a in args[:4])
                label = f"{display} {arg_str}".strip() if arg_str else display
                return {
                    "kind": "action",
                    "label": _truncate(label, 80),
                    "command": display,
                    "args": [_truncate(a) for a in args[:4]],
                }
            return None

        case IRBarrier(args=args) as ir_barrier:
            canonical = ir_barrier.canonical_command
            display = ir_barrier.command
            if REGISTRY.is_diagram_action(canonical):
                arg_str = " ".join(_truncate(a) for a in args[:4])
                label = f"{display} {arg_str}".strip() if arg_str else display
                return {
                    "kind": "action",
                    "label": _truncate(label, 80),
                    "command": display,
                    "args": [_truncate(a) for a in args[:4]],
                }
            return None

        case IRReturn(value=value):
            label = "return"
            if value:
                label += f" {_truncate(value)}"
            return {"kind": "return", "label": label}

        case IRAssignConst(name=name, value=value) if _is_notable_assign(value):
            return {
                "kind": "assign",
                "var": name,
                "value": _truncate(value, 80),
            }

        case IRAssignValue(name=name, value=value) if _is_notable_assign(value):
            return {
                "kind": "assign",
                "var": name,
                "value": _truncate(value, 80),
            }

        case IRAssignExpr(name=name, expr=expr):
            text = expr_text(expr)
            if _is_notable_assign(text):
                return {
                    "kind": "assign",
                    "var": name,
                    "value": _truncate(text, 80),
                }
            return None

        case IRCatch(body=body):
            child = _walk_script(body, proc_names, depth + 1)
            return {"kind": "catch", "body": child}

        case IRTry(body=body, handlers=handlers, finally_body=finally_body):
            child = _walk_script(body, proc_names, depth + 1)
            handler_nodes: list[dict] = []
            for handler in handlers:
                h_body = _walk_script(handler.body, proc_names, depth + 1)
                handler_nodes.append(
                    {
                        "kind_handler": handler.kind,
                        "match": handler.match_arg,
                        "body": h_body,
                    }
                )
            result: dict = {"kind": "try", "body": child}
            if handler_nodes:
                result["handlers"] = handler_nodes
            if finally_body is not None:
                result["finally"] = _walk_script(finally_body, proc_names, depth + 1)
            return result

    return None


def _walk_script(
    script: IRScript,
    proc_names: frozenset[str],
    depth: int,
) -> list[dict]:
    """Walk all statements in an IRScript, returning non-None flow nodes."""
    nodes: list[dict] = []
    for stmt in script.statements:
        node = _walk_statement(stmt, proc_names, depth)
        if node is not None:
            nodes.append(node)
    return nodes


# Public API


def extract_diagram_data(source: str) -> dict:
    """Extract structured flow data from iRule source for diagram generation.

    Returns a dict with ``events`` and ``procedures`` keys suitable for
    JSON serialisation.
    """
    ir_module = lower_to_ir(source)

    # Separate event handlers from regular procedures.
    event_procs: dict[str, IRProcedure] = {}
    regular_procs: dict[str, IRProcedure] = {}
    for key, proc in ir_module.procedures.items():
        if key.startswith("::when::"):
            event_procs[key] = proc
        else:
            regular_procs[key] = proc

    # Build set of user-defined procedure names for call detection.
    proc_names = frozenset(proc.name for proc in regular_procs.values())

    # Order events by canonical firing order, then by priority within each.
    event_names = frozenset(when_event_name(k) for k in event_procs)
    ordered = EVENT_REGISTRY.order_events(event_names)

    # Group handlers per event, sorted by base priority then file order.
    from collections import defaultdict

    handlers_by_event: dict[str, list[IRProcedure]] = defaultdict(list)
    for proc in event_procs.values():
        handlers_by_event[when_event_name(proc.qualified_name)].append(proc)
    for handlers in handlers_by_event.values():
        handlers.sort(key=lambda p: p.base_priority)

    # Walk each event handler.
    events: list[dict] = []
    for event_name in ordered:
        for proc in handlers_by_event.get(event_name, []):
            flow = _walk_script(proc.body, proc_names, depth=0)
            base_pri: int | None = proc.base_priority if proc.base_priority != 500 else None
            events.append(
                {
                    "name": event_name,
                    "priority": base_pri,
                    "multiplicity": _multiplicity(event_name),
                    "flow": flow,
                }
            )
            if len(events) >= _MAX_EVENTS:
                break
        if len(events) >= _MAX_EVENTS:
            break

    # Walk regular procedures.
    procedures: list[dict] = []
    for proc in regular_procs.values():
        flow = _walk_script(proc.body, proc_names, depth=0)
        procedures.append(
            {
                "name": proc.name,
                "params": list(proc.params),
                "flow": flow,
            }
        )

    return {
        "events": events,
        "procedures": procedures,
    }
