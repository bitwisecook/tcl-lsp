# Enriched from F5 iRules reference documentation.
"""ASM::violation -- Returns the list of violations found in the request or response together with details on each one."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ASM__violation.html"


_av = make_av(_SOURCE)


@register
class AsmViolationCommand(CommandDef):
    name = "ASM::violation"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ASM::violation",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the list of violations found in the request or response together with details on each one.",
                synopsis=("ASM::violation (count | names | attack_types | details | rating)",),
                snippet="Returns the list of violations found in the request or response together with details on each one.",
                source=_SOURCE,
                examples=(
                    "when ASM_REQUEST_DONE {\n"
                    "  set i 0\n"
                    "  foreach {viol} [ASM::violation names]{\n"
                    '    if {$viol eq "VIOLATION_ILLEGAL_PARAMETER"} {\n'
                    "      set details [lindex [ASM::violation details] $i]\n"
                    '      set param_name [b64_decode [llookup $details "param_data.param_name"]]\n'
                    "      #remove the bad parameter from the QS - does not work right in all cases,just for illustration!\n"
                    '      regsub -all "\\?.*($param_name=[^\\&]*)" [HTTP::uri] "?" $new_uri\n'
                    "      HTTP::uri $new_uri\n"
                    "      ASM::unblock\n"
                    "    }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ASM::violation (count | names | attack_types | details | rating)",
                    arg_values={
                        0: (
                            _av(
                                "count",
                                "ASM::violation count",
                                "ASM::violation (count | names | attack_types | details | rating)",
                            ),
                            _av(
                                "names",
                                "ASM::violation names",
                                "ASM::violation (count | names | attack_types | details | rating)",
                            ),
                            _av(
                                "attack_types",
                                "ASM::violation attack_types",
                                "ASM::violation (count | names | attack_types | details | rating)",
                            ),
                            _av(
                                "details",
                                "ASM::violation details",
                                "ASM::violation (count | names | attack_types | details | rating)",
                            ),
                            _av(
                                "rating",
                                "ASM::violation rating",
                                "ASM::violation (count | names | attack_types | details | rating)",
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
