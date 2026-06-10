# Enriched from F5 iRules reference documentation.
"""peer -- Causes the specified iRule commands to be evaluated under the peer-side context."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import ArgRole, Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/peer.html"


def _peer_arg_roles(args: list[str]) -> dict[int, frozenset[ArgRole]]:
    """The nesting script is a body evaluated in the peer-side context.

    ``peer`` is the third side-switch (alongside ``clientside`` /
    ``serverside``); the script is the sole argument (index 0) and runs
    synchronously in the caller's scope (``BodyKind.INLINE``).
    """
    if args:
        return {0: frozenset({ArgRole.BODY})}
    return {}


@register
class PeerCommand(CommandDef):
    name = "peer"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="peer",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Causes the specified iRule commands to be evaluated under the peer-side context.",
                synopsis=("peer NESTING_SCRIPT",),
                snippet="Causes the specified iRule commands to be evaluated under the peer-side context.",
                source=_SOURCE,
                examples=("when SERVER_CONNECTED {\n  peer { TCP::collect }\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="peer NESTING_SCRIPT",
                ),
            ),
            validation=ValidationSpec(
                # ``peer NESTING_SCRIPT`` — unlike clientside/serverside, peer
                # has no bare query form, so the script body is required:
                # exactly one argument at index 0.
                arity=Arity(1, 1),
            ),
            arg_role_resolver=_peer_arg_roles,
            is_side_switch=True,
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
