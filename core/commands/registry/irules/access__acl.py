# Enriched from F5 iRules reference documentation.
"""ACCESS::acl -- Poll or enforce ACLs in your connections."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ACCESS__acl.html"


_av = make_av(_SOURCE)


@register
class AccessAclCommand(CommandDef):
    name = "ACCESS::acl"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ACCESS::acl",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Poll or enforce ACLs in your connections.",
                synopsis=("ACCESS::acl (result | matched | lookup | (eval ACL_NAME))",),
                snippet=(
                    "The ACCESS::acl commands allow you to poll, query or enforce ACLs for a\n"
                    "given connection.\n"
                    "\n"
                    "ACCESS::acl result\n"
                    "\n"
                    "     * Returns the result of ACL match for a particular URI in\n"
                    "       ACCESS_ACL_ALLOWED and ACCESS_ACL_DENIED events.\n"
                    "     * This result can have one of the following values\n"
                    "     * - Allow\n"
                    "     * - Reject\n"
                    "\n"
                    "ACCESS::acl lookup\n"
                    "\n"
                    "     * Returns the name of all the assigned ACLs for a particular session.\n"
                    "\n"
                    "ACCESS::acl eval $acl_name\n"
                    "\n"
                    "     * Allows admin to enforce an ACL to a user request from iRule."
                ),
                source=_SOURCE,
                examples=('when ACCESS_ACL_ALLOWED {\n      ACCESS::acl eval "additional_acl"\n}'),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ACCESS::acl (result | matched | lookup | (eval ACL_NAME))",
                    arg_values={
                        0: (
                            _av(
                                "eval",
                                "ACCESS::acl eval",
                                "ACCESS::acl (result | matched | lookup | (eval ACL_NAME))",
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
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
