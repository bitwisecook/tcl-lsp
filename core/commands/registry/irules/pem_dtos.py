# Enriched from F5 iRules reference documentation.
"""pem_dtos -- Queries DTOS (Device Type and OS) database."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/pem_dtos.html"


_av = make_av(_SOURCE)


@register
class PemDtosCommand(CommandDef):
    name = "pem_dtos"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="pem_dtos",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Queries DTOS (Device Type and OS) database.",
                synopsis=("pem_dtos 'tac' 'lookup' PEM_DTOS_MCRO",),
                snippet="Queries DTOS (Device Type and OS) database",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="pem_dtos 'tac' 'lookup' PEM_DTOS_MCRO",
                    arg_values={
                        0: (
                            _av("tac", "pem_dtos tac", "pem_dtos 'tac' 'lookup' PEM_DTOS_MCRO"),
                            _av(
                                "lookup", "pem_dtos lookup", "pem_dtos 'tac' 'lookup' PEM_DTOS_MCRO"
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
