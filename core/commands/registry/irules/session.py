# Enriched from F5 iRules reference documentation.
"""session -- Utilizes the persistence table to store arbitrary information based on the same keys as persistence."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget, StorageScope
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, SubCommand, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/session.html"


_av = make_av(_SOURCE)


@register
class SessionCommand(CommandDef):
    name = "session"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="session",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Utilizes the persistence table to store arbitrary information based on the same keys as persistence.",
                synopsis=(
                    "session add SESSION_MODE",
                    "session (lookup | delete) SESSION_MODE",
                ),
                snippet=(
                    "Utilizes the persistence table to store arbitrary information based on\n"
                    "the same keys as persistence. This information does not affect the\n"
                    "persistence itself."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "set value [session lookup uie [list $myVar any virtual]]\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="session add SESSION_MODE",
                    arg_values={
                        0: (
                            _av(
                                "lookup", "session lookup", "session (lookup | delete) SESSION_MODE"
                            ),
                            _av(
                                "delete", "session delete", "session (lookup | delete) SESSION_MODE"
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                client_side=True, flow=True, also_in=frozenset({"PERSIST_DOWN"})
            ),
            xc_translatable=False,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.PERSISTENCE_TABLE,
                    reads=True,
                    writes=True,
                    scope=StorageScope.PERSISTENCE,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
            subcommands={
                "add": SubCommand(
                    name="add",
                    arity=Arity(),
                    mutator=True,
                    side_effect_hints=(
                        SideEffect(
                            target=SideEffectTarget.PERSISTENCE_TABLE,
                            reads=False,
                            writes=True,
                            scope=StorageScope.PERSISTENCE,
                            connection_side=ConnectionSide.CLIENT,
                        ),
                    ),
                ),
                "lookup": SubCommand(
                    name="lookup",
                    arity=Arity(),
                    pure=True,
                    side_effect_hints=(
                        SideEffect(
                            target=SideEffectTarget.PERSISTENCE_TABLE,
                            reads=True,
                            writes=False,
                            scope=StorageScope.PERSISTENCE,
                            connection_side=ConnectionSide.CLIENT,
                        ),
                    ),
                ),
                "delete": SubCommand(
                    name="delete",
                    arity=Arity(),
                    mutator=True,
                    side_effect_hints=(
                        SideEffect(
                            target=SideEffectTarget.PERSISTENCE_TABLE,
                            reads=False,
                            writes=True,
                            scope=StorageScope.PERSISTENCE,
                            connection_side=ConnectionSide.CLIENT,
                        ),
                    ),
                ),
                "count": SubCommand(
                    name="count",
                    arity=Arity(),
                    pure=True,
                    side_effect_hints=(
                        SideEffect(
                            target=SideEffectTarget.PERSISTENCE_TABLE,
                            reads=True,
                            writes=False,
                            scope=StorageScope.PERSISTENCE,
                            connection_side=ConnectionSide.CLIENT,
                        ),
                    ),
                ),
            },
        )
