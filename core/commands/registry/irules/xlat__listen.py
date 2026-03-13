# Enriched from F5 iRules reference documentation.
"""XLAT::listen -- Creates a related ephemeral listener."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/XLAT__listen.html"


@register
class XlatListenCommand(CommandDef):
    name = "XLAT::listen"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="XLAT::listen",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Creates a related ephemeral listener.",
                synopsis=(
                    "XLAT::listen (-hairpin)? (-inherit-main-rules)? (-single-connection)? (-translation-loose)? (XLAT_LISTEN_SUBCMDS)+",
                ),
                snippet="Creates a related ephemeral listener and returns the TCL handle for the listener. bind address and port can be omitted. It is recommend that users don't set this, so the command can choose an IP:port based on the server address specified and also conforms to source translation config. If the server address is on the clientside, then bind IP::port will be a valid endpoint on the clientside and conforms to the source translation config on the clientside.",
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    "    set listen [XLAT::listen -inherit-main-rules 30 {\n"
                    "        proto [IP::protocol]\n"
                    "        bind -allow [LINK::vlan_id],/Common/public1 -ip [serverside {IP::local_addr}]\n"
                    "        server [IP::client_addr] 7000\n"
                    "        allow [LB::server addr] 0\n"
                    "        inherit-vs [virtual]\n"
                    "    }]\n"
                    '    log local0. "LISTEN: $listen"\n'
                    "\n"
                    "    # hairpin\n"
                    "    set listen_hairpin [XLAT::listen -hairpin 30 {\n"
                    "        proto [IP::protocol]\n"
                    "        bind -allow [clientside {LINK::vlan_id}]"
                ),
                return_value='Return the TCL handle to the created listener. String representaion of the handle: "<local addr>%<local route domain id>,<local port>,<remote addr>%<remote route domain id>,<remote port>,<server addr>%<server route domain id>,<server port>,<vlan id>,<protocol number>".',
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="XLAT::listen (-hairpin)? (-inherit-main-rules)? (-single-connection)? (-translation-loose)? (XLAT_LISTEN_SUBCMDS)+",
                    options=(
                        OptionSpec(name="-hairpin", detail="Option -hairpin.", takes_value=False),
                        OptionSpec(
                            name="-inherit-main-rules",
                            detail="Option -inherit-main-rules.",
                            takes_value=False,
                        ),
                        OptionSpec(
                            name="-single-connection",
                            detail="Option -single-connection.",
                            takes_value=False,
                        ),
                        OptionSpec(
                            name="-translation-loose",
                            detail="Option -translation-loose.",
                            takes_value=False,
                        ),
                    ),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                also_in=frozenset({"CLIENT_DATA", "SERVER_CONNECTED", "SERVER_DATA"})
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.LSN_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
