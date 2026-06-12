"""Registry-driven generators for behavioural front-end test cases.

These turn the command and event registries into concrete source
snippets paired with the diagnostics the front-end must (or must not)
emit — so the behavioural suite is generated from the registry, stays
comprehensive automatically, and uses the registry as its own oracle.

Each generator yields :class:`_harness.DiagCase` values.  The predicates
were validated empirically against the live ``tcl diag`` front-end (see
the design contract) so the generated expectations are reliable, not
guesses: e.g. the "command used outside its valid events fires
IRULE1001" predicate held for 120/120 sampled commands.
"""

from __future__ import annotations

from collections.abc import Iterator

from compiler.registry import REGISTRY
from compiler.registry.info import lookup_command_info

from ._harness import DiagCase

# Diagnostic codes the generators reason about.
E_NO_SUBCOMMAND = "E001"
E_TOO_FEW = "E002"
E_TOO_MANY = "E003"
W_UNKNOWN_SUBCOMMAND = "W001"
IRULE_EVENT_SCOPING = "IRULE1001"
IRULE_UNKNOWN_EVENT = "IRULE1002"

_UNLIMITED = 2**40


def _is_simple_positional(spec, dialect: str) -> bool:
    """A command whose arity is plain positional (no subcommands, options, body).

    Restricting the arity generators to this subset keeps their
    expectations exact: option flags, subcommand words, and body
    arguments all perturb the raw argument count the arity checker sees.
    """
    if spec.subcommands or spec.switch_names(dialect):
        return False
    if spec.is_control_flow or spec.has_loop_body or spec.body_kind.name != "INLINE":
        return False
    if "::" in spec.name:
        return False
    return True


def arity_cases(dialect: str) -> Iterator[DiagCase]:
    """Under-arity (E002), over-arity (E003), and minimal-valid cases."""
    for name in REGISTRY.command_names(dialect):
        spec = REGISTRY.get(name, dialect)
        if spec is None or not _is_simple_positional(spec, dialect):
            continue
        validation = REGISTRY.validation(name, dialect)
        if validation is None:
            continue
        arity = validation.arity

        if arity.min > 0:
            short = f"{name} " + " ".join(["x"] * (arity.min - 1))
            yield DiagCase(
                cid=f"arity-too-few:{dialect}:{name}",
                source=short + "\n",
                dialect=dialect,
                must_have=frozenset({E_TOO_FEW}),
            )

        if arity.max < _UNLIMITED:
            over = f"{name} " + " ".join(["x"] * (arity.max + 1))
            yield DiagCase(
                cid=f"arity-too-many:{dialect}:{name}",
                source=over + "\n",
                dialect=dialect,
                must_have=frozenset({E_TOO_MANY}),
            )

        # A minimal valid call must raise no arity diagnostic.
        valid = f"{name} " + " ".join(["x"] * arity.min)
        yield DiagCase(
            cid=f"arity-valid:{dialect}:{name}",
            source=valid + "\n",
            dialect=dialect,
            must_not=frozenset({E_TOO_FEW, E_TOO_MANY, E_NO_SUBCOMMAND}),
        )


def subcommand_cases(dialect: str) -> Iterator[DiagCase]:
    """Bare ensemble (E001), bogus subcommand (W001), and valid subcommand."""
    for name in REGISTRY.command_names(dialect):
        spec = REGISTRY.get(name, dialect)
        if spec is None or not spec.subcommands or "::" in name:
            continue
        subs = REGISTRY.subcommand_names(name, dialect)
        if not subs:
            continue

        # A bare ensemble call must report "requires a subcommand".
        yield DiagCase(
            cid=f"ensemble-bare:{dialect}:{name}",
            source=f"{name}\n",
            dialect=dialect,
            must_have=frozenset({E_NO_SUBCOMMAND}),
        )

        if not spec.allow_unknown_subcommands:
            yield DiagCase(
                cid=f"ensemble-bogus:{dialect}:{name}",
                source=f"{name} __no_such_sub__\n",
                dialect=dialect,
                must_have=frozenset({W_UNKNOWN_SUBCOMMAND}),
            )

        # A known subcommand must not be reported as unknown / missing.
        sub = subs[0]
        yield DiagCase(
            cid=f"ensemble-valid:{dialect}:{name}:{sub}",
            source=f"{name} {sub} x y z\n",
            dialect=dialect,
            must_not=frozenset({W_UNKNOWN_SUBCOMMAND, E_NO_SUBCOMMAND}),
        )


def _when(event: str, body: str) -> str:
    # Explicit priority avoids the unrelated IRULE1004 "missing priority" note.
    return f"when {event} priority 500 {{ {body} }}\n"


def event_scoping_cases(
    dialect: str = "f5-irules", *, limit: int | None = None
) -> Iterator[DiagCase]:
    """iRules event-scoping (IRULE1001): commands used outside their valid events.

    Positive cases (must warn) use the validated predicate "command has
    event requirements and is used in an event outside its
    ``command-info`` valid-events".  Negative cases use any-event
    commands (no requirements), which must never warn.
    """
    events = sorted({v.value for v in REGISTRY.argument_values("when", 0, dialect)})
    event_set = set(events)
    emitted = 0
    for name in REGISTRY.command_names(dialect):
        if "::" not in name:
            continue
        info = lookup_command_info(name, dialect=dialect)
        if info.valid_in_any_event:
            # Any-event command: must not warn anywhere.  Probe one event.
            yield DiagCase(
                cid=f"event-any:{name}",
                source=_when(events[0], name),
                dialect=dialect,
                must_not=frozenset({IRULE_EVENT_SCOPING}),
            )
        else:
            valid = set(info.valid_events) & event_set
            outside = sorted(event_set - valid)
            if not valid or not outside:
                continue
            # Used outside its valid events -> must warn.
            yield DiagCase(
                cid=f"event-outside:{name}:{outside[0]}",
                source=_when(outside[0], name),
                dialect=dialect,
                must_have=frozenset({IRULE_EVENT_SCOPING}),
            )
        emitted += 1
        if limit is not None and emitted >= limit:
            return


def unknown_event_case(dialect: str = "f5-irules") -> DiagCase:
    return DiagCase(
        cid="event-unknown",
        source="when __NOT_A_REAL_EVENT__ priority 500 { set x 1 }\n",
        dialect=dialect,
        must_have=frozenset({IRULE_UNKNOWN_EVENT}),
    )
