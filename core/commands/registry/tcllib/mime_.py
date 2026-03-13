"""mime -- MIME handling (tcllib)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "tcllib mime package"
_PACKAGE = "mime"


@register
class MimeInitialiseCommand(CommandDef):
    name = "mime::initialize"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Create a MIME part from data, a file, or a channel.",
                synopsis=(
                    "mime::initialize ?-canonical type? ?-params dict? "
                    "?-encoding enc? ?-header dict? "
                    "?-string text | -file path | -chan channel?",
                ),
                source=_SOURCE,
                return_value="A MIME token.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="mime::initialize ?options?",
                ),
            ),
            validation=ValidationSpec(arity=Arity(0)),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )


@register
class MimeGetpropertyCommand(CommandDef):
    name = "mime::getproperty"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Return a property of a MIME part.",
                synopsis=("mime::getproperty token ?property?",),
                source=_SOURCE,
                return_value="The property value or a key-value list of all properties.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="mime::getproperty token ?property?",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1, 2)),
        )


@register
class MimeGetbodyCommand(CommandDef):
    name = "mime::getbody"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Return the body of a MIME part.",
                synopsis=("mime::getbody token ?-decode? ?-command cmdprefix?",),
                source=_SOURCE,
                return_value="The body content.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="mime::getbody token ?options?",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1)),
        )


@register
class MimeFinaliseCommand(CommandDef):
    name = "mime::finalize"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Destroy a MIME part and free resources.",
                synopsis=("mime::finalize token ?-subordinates all?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="mime::finalize token ?-subordinates all?",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1, 3)),
        )
