# Enriched from F5 iRules reference documentation.
"""ACCESS::respond -- This command generates new respond and automatically overrides the default respond."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionSpec,
    ValidationSpec,
)
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ACCESS__respond.html"


_av = make_av(_SOURCE)


@register
class AccessRespondCommand(CommandDef):
    name = "ACCESS::respond"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ACCESS::respond",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command generates new respond and automatically overrides the default respond.",
                synopsis=(
                    "ACCESS::respond STATUS_CODE (ifile | -ifile) IFILE_OBJ",
                    "ACCESS::respond STATUS_CODE (((('content' | '-content') CONTENT)",
                ),
                snippet=(
                    "This command generates new respond and automatically overrides the\n"
                    "default respond. This command only can be used only once per HTTP\n"
                    "request, and subsequent calls to this command will return an error.\n"
                    "\n"
                    "HTTP iRules should be used with caution after an ACCESS::respond call.\n"
                    "They may not behave as expected since ACCESS::respond creates an HTTP response.\n"
                    "As of version 13.0.0, the way that HTTP caching interacts with the\n"
                    "HTTP iRule commands has changed, so inconsistencies are expected when using\n"
                    "HTTP iRules after ACCESS::respond."
                ),
                source=_SOURCE,
                examples=(
                    "when ACCESS_POLICY_COMPLETED {\n"
                    "    set policy_result [ACCESS::policy result]\n"
                    "    switch $policy_result {\n"
                    '    "allow" {\n'
                    "    # Do nothing\n"
                    "    }\n"
                    '    "deny" {\n'
                    '        ACCESS::respond 401 content "<html><body>Error: Failure in Authentication</body></html>" Connection Close\n'
                    "    }\n"
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ACCESS::respond STATUS_CODE (ifile | -ifile) IFILE_OBJ",
                    options=(
                        OptionSpec(name="-ifile", detail="Option -ifile.", takes_value=False),
                        OptionSpec(name="-content", detail="Option -content.", takes_value=True),
                    ),
                    arg_values={
                        0: (
                            _av(
                                "ifile",
                                "ACCESS::respond ifile",
                                "ACCESS::respond STATUS_CODE (ifile | -ifile) IFILE_OBJ",
                            ),
                            _av(
                                "content",
                                "ACCESS::respond content",
                                "ACCESS::respond STATUS_CODE (((('content' | '-content') CONTENT)",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"ACCESS"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.APM_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
