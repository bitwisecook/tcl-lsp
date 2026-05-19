"""logger -- Logging facility (tcllib)."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

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


@register
class LoggerEnableCommand(CommandDef):
    name = "logger::enable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Enable logging at the specified level.",
                synopsis=("logger::enable level",),
                source=_SOURCE,
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="logger::enable level"),),
            validation=ValidationSpec(arity=Arity(1, 1)),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )


@register
class LoggerDisableCommand(CommandDef):
    name = "logger::disable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Disable logging at the specified level.",
                synopsis=("logger::disable level",),
                source=_SOURCE,
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="logger::disable level"),),
            validation=ValidationSpec(arity=Arity(1, 1)),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )


@register
class LoggerImportCommand(CommandDef):
    name = "logger::import"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Import logger commands into the current namespace.",
                synopsis=(
                    "logger::import ?-all? ?-force? ?-prefix prefix? ?-namespace namespace? service",
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="logger::import ?-all? ?-force? ?-prefix prefix? ?-namespace namespace? service",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1)),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )


@register
class LoggerWalkCommand(CommandDef):
    name = "logger::walk"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Walk the logger tree applying a command to each service.",
                synopsis=("logger::walk service command",),
                source=_SOURCE,
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="logger::walk service command"),),
            validation=ValidationSpec(arity=Arity(2, 2)),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )


@register
class LoggerSetlevelCommand(CommandDef):
    name = "logger::setlevel"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Set the logging level for all logger services.",
                synopsis=("logger::setlevel level",),
                source=_SOURCE,
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="logger::setlevel level"),),
            validation=ValidationSpec(arity=Arity(1, 1)),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )


@register
class LoggerInitNamespaceCommand(CommandDef):
    name = "logger::initNamespace"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Initialise logging for a namespace.",
                synopsis=("logger::initNamespace ns ?level?",),
                source=_SOURCE,
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="logger::initNamespace ns ?level?"),),
            validation=ValidationSpec(arity=Arity(1, 2)),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
