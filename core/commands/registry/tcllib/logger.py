"""logger -- Logging facility (tcllib)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "tcllib logger package"
_PACKAGE = "logger"


@register
class LoggerInitCommand(CommandDef):
    name = "logger::init"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Initialise a logger for a service.",
                synopsis=("logger::init service",),
                snippet=(
                    "Creates a new logger instance for the given service "
                    "name. Returns a logger command for controlling "
                    "log levels and output."
                ),
                source=_SOURCE,
                examples="set log [logger::init myapp]",
                return_value="A logger command token.",
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="logger::init service"),),
            validation=ValidationSpec(arity=Arity(1, 1)),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.LOG_IO, writes=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )


@register
class LoggerServicesCommand(CommandDef):
    name = "logger::services"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Return a list of all active logger services.",
                synopsis=("logger::services",),
                source=_SOURCE,
                return_value="A list of service names.",
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="logger::services"),),
            validation=ValidationSpec(arity=Arity(0, 0)),
        )


@register
class LoggerLevelsCommand(CommandDef):
    name = "logger::levels"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Return the list of valid log levels.",
                synopsis=("logger::levels",),
                source=_SOURCE,
                return_value=("The list: debug info notice warn error critical alert emergency."),
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="logger::levels"),),
            validation=ValidationSpec(arity=Arity(0, 0)),
            pure=True,
        )


@register
class LoggerServicecmdCommand(CommandDef):
    name = "logger::servicecmd"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Return the command token for a named logger service.",
                synopsis=("logger::servicecmd service",),
                source=_SOURCE,
                return_value="The logger command for the named service.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="logger::servicecmd service",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1, 1)),
        )
