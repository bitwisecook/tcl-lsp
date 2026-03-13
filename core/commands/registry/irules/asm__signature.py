# Enriched from F5 iRules reference documentation.
"""ASM::signature -- Returns the list of signatures."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ASM__signature.html"


_av = make_av(_SOURCE)


@register
class AsmSignatureCommand(CommandDef):
    name = "ASM::signature"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ASM::signature",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the list of signatures.",
                synopsis=(
                    "ASM::signature (ids | names | set_names | staged_ids | staged_names | staged_set_names)",
                ),
                snippet="Returns the list of signatures.",
                source=_SOURCE,
                examples=(
                    "when ASM_REQUEST_DONE {\n"
                    '    log local0. "ids=[ASM::signature ids] names=[ASM::signature names] set_names=[ASM::signature set_names]"\n'
                    '    log local0. "staged_ids=[ASM::signature staged_ids] staged_names=[ASM::signature staged_names] staged_set_names=[ASM::signature staged_set_names]"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ASM::signature (ids | names | set_names | staged_ids | staged_names | staged_set_names)",
                    arg_values={
                        0: (
                            _av(
                                "ids",
                                "ASM::signature ids",
                                "ASM::signature (ids | names | set_names | staged_ids | staged_names | staged_set_names)",
                            ),
                            _av(
                                "names",
                                "ASM::signature names",
                                "ASM::signature (ids | names | set_names | staged_ids | staged_names | staged_set_names)",
                            ),
                            _av(
                                "set_names",
                                "ASM::signature set_names",
                                "ASM::signature (ids | names | set_names | staged_ids | staged_names | staged_set_names)",
                            ),
                            _av(
                                "staged_ids",
                                "ASM::signature staged_ids",
                                "ASM::signature (ids | names | set_names | staged_ids | staged_names | staged_set_names)",
                            ),
                            _av(
                                "staged_names",
                                "ASM::signature staged_names",
                                "ASM::signature (ids | names | set_names | staged_ids | staged_names | staged_set_names)",
                            ),
                            _av(
                                "staged_set_names",
                                "ASM::signature staged_set_names",
                                "ASM::signature (ids | names | set_names | staged_ids | staged_names | staged_set_names)",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"ASM"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
