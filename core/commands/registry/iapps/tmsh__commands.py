"""tmsh:: namespace commands for iApp/tmsh scripting.

Registers all ``tmsh::`` commands available in iApp implementation scripts
and tmsh CLI scripts.  Reference:
https://clouddocs.f5.com/api/tmsh/Commands.html
"""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import _TMSH_DIALECTS, register

_SOURCE = "F5 TMSH Scripting Reference"


def _tmsh_spec(
    name: str,
    summary: str,
    synopsis: str = "",
    *,
    min_args: int = 0,
    max_args: int | None = None,
) -> CommandSpec:
    """Build a CommandSpec for a tmsh:: command."""
    if not synopsis:
        synopsis = f"{name} ?arg ...?"
    arity = Arity(min=min_args) if max_args is None else Arity(min=min_args, max=max_args)
    return CommandSpec(
        name=name,
        dialects=_TMSH_DIALECTS,
        hover=HoverSnippet(
            summary=summary,
            synopsis=(synopsis,),
            source=_SOURCE,
        ),
        forms=(FormSpec(kind=FormKind.DEFAULT, synopsis=synopsis),),
        validation=ValidationSpec(arity=arity),
    )


# --- tmsh:: configuration commands ---


@register
class TmshCreate(CommandDef):
    name = "tmsh::create"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::create",
            "Mirrors the tmsh ``create`` command.",
            "tmsh::create <component> <name> ?options?",
            min_args=2,
        )


@register
class TmshModify(CommandDef):
    name = "tmsh::modify"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::modify",
            "Runs the ``modify`` command using the specified arguments.",
            "tmsh::modify <component> <name> ?options?",
            min_args=2,
        )


@register
class TmshDelete(CommandDef):
    name = "tmsh::delete"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::delete",
            "Mirrors the tmsh ``delete`` command.",
            "tmsh::delete <component> <name>",
            min_args=2,
        )


@register
class TmshList(CommandDef):
    name = "tmsh::list"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::list",
            "Runs the ``list`` command using the specified arguments.",
            "tmsh::list ?component? ?name? ?options?",
        )


@register
class TmshShow(CommandDef):
    name = "tmsh::show"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::show",
            "Runs the ``show`` command using the specified arguments.",
            "tmsh::show ?component? ?name? ?options?",
        )


# --- tmsh:: object introspection ---


@register
class TmshGetConfig(CommandDef):
    name = "tmsh::get_config"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::get_config",
            "Returns a list of configuration items as Tcl objects.",
            "tmsh::get_config <component> ?name? ?options?",
            min_args=1,
        )


@register
class TmshGetStatus(CommandDef):
    name = "tmsh::get_status"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::get_status",
            "Returns a list of config item statuses as Tcl objects.",
            "tmsh::get_status <component> ?name? ?options?",
            min_args=1,
        )


@register
class TmshGetFieldNames(CommandDef):
    name = "tmsh::get_field_names"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::get_field_names",
            "Returns a list of field names present in an object.",
            "tmsh::get_field_names <object>",
            min_args=1,
            max_args=1,
        )


@register
class TmshGetFieldValue(CommandDef):
    name = "tmsh::get_field_value"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::get_field_value",
            "Retrieves the value of the field name.",
            "tmsh::get_field_value <object> <field_name>",
            min_args=2,
            max_args=2,
        )


@register
class TmshGetName(CommandDef):
    name = "tmsh::get_name"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::get_name",
            "Returns the object identifier associated with the object.",
            "tmsh::get_name <object>",
            min_args=1,
            max_args=1,
        )


@register
class TmshGetType(CommandDef):
    name = "tmsh::get_type"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::get_type",
            "Returns the type identifier associated with the object.",
            "tmsh::get_type <object>",
            min_args=1,
            max_args=1,
        )


# --- tmsh:: transaction commands ---


@register
class TmshBeginTransaction(CommandDef):
    name = "tmsh::begin_transaction"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::begin_transaction",
            "Begins an update transaction.",
            "tmsh::begin_transaction",
            max_args=0,
        )


@register
class TmshCommitTransaction(CommandDef):
    name = "tmsh::commit_transaction"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::commit_transaction",
            "Runs all commands issued since the last ``tmsh::begin_transaction``.",
            "tmsh::commit_transaction",
            max_args=0,
        )


@register
class TmshCancelTransaction(CommandDef):
    name = "tmsh::cancel_transaction"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::cancel_transaction",
            "Cancels all commands issued since the last ``tmsh::begin_transaction``.",
            "tmsh::cancel_transaction",
            max_args=0,
        )


# --- tmsh:: logging and display ---


@register
class TmshLog(CommandDef):
    name = "tmsh::log"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::log",
            "Logs the specified message.",
            "tmsh::log <message>",
            min_args=1,
        )


@register
class TmshLogDest(CommandDef):
    name = "tmsh::log_dest"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::log_dest",
            "Specifies where the system sends events.",
            "tmsh::log_dest ?destination?",
        )


@register
class TmshLogLevel(CommandDef):
    name = "tmsh::log_level"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::log_level",
            "Specifies the default severity level.",
            "tmsh::log_level ?level?",
        )


@register
class TmshDisplay(CommandDef):
    name = "tmsh::display"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::display",
            "Provides access to the tmsh pager.",
            "tmsh::display <text>",
            min_args=1,
        )


@register
class TmshDisplayThreshold(CommandDef):
    name = "tmsh::display_threshold"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::display_threshold",
            "Re-enables a display-threshold in the script.",
            "tmsh::display_threshold ?value?",
        )


@register
class TmshClearScreen(CommandDef):
    name = "tmsh::clear_screen"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::clear_screen",
            "Clears the screen.",
            "tmsh::clear_screen",
            max_args=0,
        )


# --- tmsh:: utility commands ---


@register
class TmshInclude(CommandDef):
    name = "tmsh::include"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::include",
            "Runs the Tcl command ``eval`` on the specified script.",
            "tmsh::include <script_name>",
            min_args=1,
            max_args=1,
        )


@register
class TmshVersion(CommandDef):
    name = "tmsh::version"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::version",
            "Returns the version number of the BIG-IP system as a Tcl string.",
            "tmsh::version",
            max_args=0,
        )


@register
class TmshResetStats(CommandDef):
    name = "tmsh::reset_stats"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::reset_stats",
            "Runs the ``reset-stats`` command using the specified arguments.",
            "tmsh::reset_stats ?component? ?name?",
        )


@register
class TmshStateless(CommandDef):
    name = "tmsh::stateless"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::stateless",
            "Modifies the behaviour of ``tmsh::create`` and ``tmsh::delete``.",
            "tmsh::stateless ?enabled?",
        )


# --- tmsh:: help and tab-completion ---


@register
class TmshAddHelp(CommandDef):
    name = "tmsh::add_help"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::add_help",
            "Displays context-sensitive help when the user types ``?``.",
            "tmsh::add_help <help_data>",
            min_args=1,
        )


@register
class TmshAddTabc(CommandDef):
    name = "tmsh::add_tabc"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::add_tabc",
            "Adds tab completion datasets.",
            "tmsh::add_tabc <tabc_data>",
            min_args=1,
        )


@register
class TmshBuiltinHelp(CommandDef):
    name = "tmsh::builtin_help"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::builtin_help",
            "Presents the same results as typing ``?`` while entering a tmsh command.",
            "tmsh::builtin_help ?args?",
        )


@register
class TmshBuiltinTabc(CommandDef):
    name = "tmsh::builtin_tabc"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::builtin_tabc",
            "Displays the same tab completion results as a built-in command.",
            "tmsh::builtin_tabc ?args?",
        )


# --- tmsh:: directory commands ---


@register
class TmshCd(CommandDef):
    name = "tmsh::cd"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::cd",
            "Changes the current working directory.",
            "tmsh::cd <directory>",
            min_args=1,
            max_args=1,
        )


@register
class TmshPwd(CommandDef):
    name = "tmsh::pwd"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::pwd",
            "Returns the current working directory.",
            "tmsh::pwd",
            max_args=0,
        )


# --- script:: namespace commands ---


@register
class ScriptHelp(CommandDef):
    name = "script::help"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "script::help",
            "Scripts can provide the ``script::help`` procedure for help text.",
            "script::help",
        )


@register
class ScriptInit(CommandDef):
    name = "script::init"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "script::init",
            "Called before ``script::run``, ``script::help``, and ``script::tabc``.",
            "script::init",
        )


@register
class ScriptRun(CommandDef):
    name = "script::run"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "script::run",
            "Invoked when the tmsh ``run cli script`` command is issued.",
            "script::run",
        )


@register
class ScriptTabc(CommandDef):
    name = "script::tabc"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "script::tabc",
            "A script may provide ``script::tabc`` for tab completion.",
            "script::tabc",
        )
