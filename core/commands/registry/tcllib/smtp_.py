"""smtp -- SMTP client (tcllib)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "tcllib smtp package"
_PACKAGE = "smtp"


@register
class SmtpSendmessageCommand(CommandDef):
    name = "smtp::sendmessage"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Send an e-mail message via SMTP.",
                synopsis=(
                    "smtp::sendmessage token "
                    "?-servers list? ?-ports list? "
                    "?-username user? ?-password pass? "
                    "?-usetls bool? ?-tlspolicy cmd? "
                    "?-originator addr? ?-recipients list? "
                    "?-header {key value} ...?",
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="smtp::sendmessage token ?options?",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1)),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
