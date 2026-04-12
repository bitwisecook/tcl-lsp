# Enriched from F5 iRules reference documentation.
"""RADIUS::avp -- This command returns or adds/changes/removes RADIUS attribute-value pairs."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/RADIUS__avp.html"


_av = make_av(_SOURCE)


@register
class RadiusAvpCommand(CommandDef):
    name = "RADIUS::avp"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="RADIUS::avp",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command returns or adds/changes/removes RADIUS attribute-value pairs.",
                synopsis=(
                    "RADIUS::avp (ATTR_NAME|ATTR_CODE) (ATTR_TYPE)? ('index' INDEX)?",
                    "RADIUS::avp 'insert' (ATTR_NAME|ATTR_CODE)",
                ),
                snippet="This command returns or adds/changes/removes RADIUS attribute-value pairs. Radius profile must be applied for access to this command.",
                source=_SOURCE,
                examples=('when RULE_INIT {\n        set static::secret "linus"\n    }'),
                return_value="RADIUS::avp attr [attr_type] Returns the value of the specified RADIUS attribute. optional attr_type = ( octet | ip4 | ip6 | integer | string)",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="RADIUS::avp (ATTR_NAME|ATTR_CODE) (ATTR_TYPE)? ('index' INDEX)?",
                    arg_values={
                        0: (
                            _av(
                                "index",
                                "RADIUS::avp index",
                                "RADIUS::avp (ATTR_NAME|ATTR_CODE) (ATTR_TYPE)? ('index' INDEX)?",
                            ),
                            _av(
                                "insert",
                                "RADIUS::avp insert",
                                "RADIUS::avp 'insert' (ATTR_NAME|ATTR_CODE)",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                also_in=frozenset(
                    {
                        "CLIENT_ACCEPTED",
                        "CLIENT_CLOSED",
                        "CLIENT_DATA",
                        "SERVER_CLOSED",
                        "SERVER_CONNECTED",
                        "SERVER_DATA",
                    }
                )
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
