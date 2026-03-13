"""msgcat -- Tcl stdlib message catalogue package (package require msgcat)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_PKG = "msgcat"
_SOURCE = "Tcl stdlib msgcat package"


@register
class MsgcatMc(CommandDef):
    name = "msgcat::mc"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="msgcat::mc",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Translate a source string according to the current locale.",
                synopsis=("msgcat::mc src-string ?arg arg ...?",),
                snippet=(
                    "Looks up *src-string* in the message catalogue for the "
                    "calling namespace and current locale.  Any additional "
                    "arguments are substituted into the translated string "
                    "via ``format``."
                ),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1)),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )


@register
class MsgcatMcn(CommandDef):
    name = "msgcat::mcn"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="msgcat::mcn",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Translate a source string in a given namespace.",
                synopsis=("msgcat::mcn namespace src-string ?arg arg ...?",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(2)),
        )


@register
class MsgcatMclocale(CommandDef):
    name = "msgcat::mclocale"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="msgcat::mclocale",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Get or set the current locale for message catalogues.",
                synopsis=("msgcat::mclocale ?newLocale?",),
                snippet=(
                    "Without arguments, returns the current locale.  "
                    "With *newLocale*, sets the locale and returns the "
                    "new value."
                ),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(0, 1)),
        )


@register
class MsgcatMcpreferences(CommandDef):
    name = "msgcat::mcpreferences"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="msgcat::mcpreferences",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Return the ordered list of locale preferences.",
                synopsis=("msgcat::mcpreferences",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(0, 0)),
        )


@register
class MsgcatMcload(CommandDef):
    name = "msgcat::mcload"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="msgcat::mcload",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Load message catalogue files from a directory.",
                synopsis=("msgcat::mcload dirname",),
                snippet=(
                    "Searches *dirname* for files matching the current "
                    "locale preferences (e.g. ``en_gb.msg``, ``en.msg``, "
                    "``ROOT.msg``) and sources them."
                ),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1, 1)),
        )


@register
class MsgcatMcset(CommandDef):
    name = "msgcat::mcset"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="msgcat::mcset",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Set the translation for a string in a given locale.",
                synopsis=("msgcat::mcset locale src-string ?translate-string?",),
                snippet=(
                    "Sets the translation for *src-string* in the "
                    "calling namespace.  If *translate-string* is "
                    "omitted, *src-string* is used as the translation."
                ),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(2, 3)),
        )


@register
class MsgcatMcmset(CommandDef):
    name = "msgcat::mcmset"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="msgcat::mcmset",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Set translations for multiple strings in a given locale.",
                synopsis=("msgcat::mcmset locale src-trans-list",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(2, 2)),
        )


@register
class MsgcatMcflset(CommandDef):
    name = "msgcat::mcflset"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="msgcat::mcflset",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Set a translation for the locale of the current message file.",
                synopsis=("msgcat::mcflset src-string ?translate-string?",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1, 2)),
        )


@register
class MsgcatMcflmset(CommandDef):
    name = "msgcat::mcflmset"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="msgcat::mcflmset",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Set multiple translations for the locale of the current message file.",
                synopsis=("msgcat::mcflmset src-trans-list",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1, 1)),
        )


@register
class MsgcatMcexists(CommandDef):
    name = "msgcat::mcexists"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="msgcat::mcexists",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Check whether a translation exists for the given source string.",
                synopsis=("msgcat::mcexists ?-exactnamespace? ?-exactlocale? src-string",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1)),
        )


@register
class MsgcatMcmax(CommandDef):
    name = "msgcat::mcmax"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="msgcat::mcmax",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Return the maximum length of the translations of the given strings.",
                synopsis=("msgcat::mcmax ?src-string ...?",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(0)),
        )


@register
class MsgcatMcunknown(CommandDef):
    name = "msgcat::mcunknown"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="msgcat::mcunknown",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Called when a translation is not found; override for custom behaviour.",
                synopsis=("msgcat::mcunknown locale src-string ?arg ...?",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(2)),
        )


@register
class MsgcatMcloadedlocales(CommandDef):
    name = "msgcat::mcloadedlocales"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="msgcat::mcloadedlocales",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Return or manage the list of locales that have been loaded.",
                synopsis=("msgcat::mcloadedlocales subcommand",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1)),
        )


@register
class MsgcatMcpackagelocale(CommandDef):
    name = "msgcat::mcpackagelocale"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="msgcat::mcpackagelocale",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Get, set, or manage the locale for the calling package.",
                synopsis=("msgcat::mcpackagelocale subcommand ?locale?",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1)),
        )


@register
class MsgcatMcforgetpackage(CommandDef):
    name = "msgcat::mcforgetpackage"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="msgcat::mcforgetpackage",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Remove all translations for the calling package.",
                synopsis=("msgcat::mcforgetpackage",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(0, 0)),
        )


@register
class MsgcatMcpackageconfig(CommandDef):
    name = "msgcat::mcpackageconfig"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="msgcat::mcpackageconfig",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Get or set per-package configuration options.",
                synopsis=("msgcat::mcpackageconfig subcommand option ?value?",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(2)),
        )
