# Enriched from F5 iRules reference documentation.
"""members -- Lists all members of a given pool for v10.x.x."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/members.html"


@register
class MembersCommand(CommandDef):
    name = "members"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="members",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Lists all members of a given pool for v10.x.x.",
                synopsis=("members ('-list')? (POOL_OBJ)",),
                snippet=(
                    "This command behaves much like active_members, but counts or lists all\n"
                    "members (IP+port combinations) in a pool, not just active ones.\n"
                    "\n"
                    "Note\n"
                    "\n"
                    '   When assigning a snatpool to static variable and using "members -list"\n'
                    "   to reference it in RULE_INIT, failures will be observed at startup but\n"
                    "   won't show up in a reload afterwards. Expected behavior is to fail it\n"
                    '   in any case as "members -list" is not designed to reference a snatpool\n'
                    "   name."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '    set response "<?xml version=\\"1.0\\" encoding=\\"utf-8\\"?><rss version=\\"2.0\\"><channel>"\n'
                    '    append response "<title>BigIP Server Pool Status</title>"\n'
                    '    append response "<description>Server Pool Status</description>"\n'
                    '    append response "<language>en</language>"\n'
                    '    append response "<pubDate>[clock format [clock seconds]]</pubDate>"\n'
                    '    append response "<ttl>60</ttl>"\n'
                    '    if { [HTTP::uri] eq "/status" } {\n'
                    "                foreach { selectedpool } [class get pooltest] {"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="members ('-list')? (POOL_OBJ)",
                    options=(OptionSpec(name="-list", detail="Option -list.", takes_value=True),),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.BIGIP_CONFIG,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
