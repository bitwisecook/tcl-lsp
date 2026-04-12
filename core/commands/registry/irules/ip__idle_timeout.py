# Enriched from F5 iRules reference documentation.
"""IP::idle_timeout -- Returns or sets the idle timeout value."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/IP__idle_timeout.html"


@register
class IpIdleTimeoutCommand(CommandDef):
    name = "IP::idle_timeout"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="IP::idle_timeout",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns or sets the idle timeout value.",
                synopsis=("IP::idle_timeout (TIMEOUT)?",),
                snippet="Returns the idle timeout value, or specifies an idle timeout value as the criteria for selecting the pool to which you want the BIG-IP system to send traffic.",
                source=_SOURCE,
                examples=("when SERVER_CONNECTED {\n    IP::idle_timeout $idle\n}"),
                return_value="idle timeout value in seconds",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="IP::idle_timeout (TIMEOUT)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
