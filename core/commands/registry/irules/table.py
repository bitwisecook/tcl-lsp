# Enriched from F5 iRules reference documentation.
"""table -- Provides enhanced access to the session table."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget, StorageScope
from .._base import CommandDef, make_av
from ..models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionSpec,
    OptionTerminatorSpec,
    SubCommand,
    ValidationSpec,
)
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/table.html"


_av = make_av(_SOURCE)

_WRITE_SUBS = frozenset({"set", "add", "replace", "incr", "append", "delete"})
_READ_SUBS = frozenset({"lookup", "keys", "timeout", "lifetime"})


@register
class TableCommand(CommandDef):
    name = "table"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="table",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Provides enhanced access to the session table.",
                synopsis=(
                    "table set (((-mustexist | -excl) -notouch ((-subtable TABLE_NAME) | -georedundancy))# ('--')?)? KEY VALUE (('indefinite' | POSITIVE_INTEGER) ('indefinite' | POSITIVE_INTEGER)?)?",
                    "table add ((-notouch ((-subtable TABLE_NAME) | -georedundancy))# ('--')?)? KEY VALUE (('indefinite' | POSITIVE_INTEGER) ('indefinite' | POSITIVE_INTEGER)?)?",
                    "table replace ((-notouch ((-subtable TABLE_NAME) | -georedundancy))# ('--')?)? KEY VALUE (('indefinite' | POSITIVE_INTEGER) ('indefinite' | POSITIVE_INTEGER)?)?",
                    "table lookup ((-notouch ((-subtable TABLE_NAME) | -georedundancy))# ('--')?)? KEY",
                ),
                snippet=(
                    "The table command is a superset of the session command, with improved syntax for general purpose use. Please see the table command article series for detailed information on its use.\n"
                    "\n"
                    "This command is not available to GTM.\n"
                    "\n"
                    "If the table command is used on the standby system in a HA pair, the command will perform a no-op because the content of the standby unit's session db should be updated only through mirroring."
                ),
                source=_SOURCE,
                examples=(
                    "when RULE_INIT {\n"
                    "    set static::maxquery 100\n"
                    "    set static::holdtime 600\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="table <subcommand> ?options? ?--? key ?value? ?lifetime? ?timeout?",
                    options=(
                        OptionSpec(
                            name="-mustexist",
                            detail="Fail if key does not already exist.",
                            takes_value=False,
                        ),
                        OptionSpec(
                            name="-excl", detail="Fail if key already exists.", takes_value=False
                        ),
                        OptionSpec(
                            name="-notouch",
                            detail="Do not reset lifetime/timeout on access.",
                            takes_value=False,
                        ),
                        OptionSpec(
                            name="-subtable",
                            detail="Operate on a named subtable.",
                            takes_value=True,
                        ),
                        OptionSpec(
                            name="-georedundancy",
                            detail="Enable geo-redundancy for this entry.",
                            takes_value=False,
                        ),
                        OptionSpec(
                            name="-remaining", detail="Return remaining time.", takes_value=False
                        ),
                        OptionSpec(
                            name="-count",
                            detail="Return count of matching keys.",
                            takes_value=False,
                        ),
                    ),
                    arg_values={
                        0: (
                            _av(
                                "set",
                                "Create or update a table entry.",
                                "table set ?options? ?--? <key> <value> ?lifetime? ?timeout?",
                            ),
                            _av(
                                "add",
                                "Add a new table entry (fail if exists).",
                                "table add ?options? ?--? <key> <value> ?lifetime? ?timeout?",
                            ),
                            _av(
                                "replace",
                                "Replace an existing table entry.",
                                "table replace ?options? ?--? <key> <value> ?lifetime? ?timeout?",
                            ),
                            _av(
                                "lookup",
                                "Look up a value by key.",
                                "table lookup ?options? ?--? <key>",
                            ),
                            _av(
                                "incr",
                                "Increment a numeric table value.",
                                "table incr ?options? ?--? <key> ?amount?",
                            ),
                            _av(
                                "append",
                                "Append to a table value.",
                                "table append ?options? ?--? <key> <string>",
                            ),
                            _av(
                                "delete",
                                "Delete a table entry.",
                                "table delete ?-subtable name? ?--? <key>",
                            ),
                            _av(
                                "timeout",
                                "Get/set entry timeout.",
                                "table timeout ?-subtable name? ?--? <key> ?value? ?-remaining?",
                            ),
                            _av(
                                "lifetime",
                                "Get/set entry lifetime.",
                                "table lifetime ?-subtable name? ?--? <key> ?value? ?-remaining?",
                            ),
                            _av(
                                "keys",
                                "List table keys.",
                                "table keys ?-subtable name? ?-notouch? ?-count? ?pattern?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            option_terminator_profiles=(
                OptionTerminatorSpec(
                    scan_start=1, options_with_values=frozenset({"-subtable"}), subcommand="set"
                ),
                OptionTerminatorSpec(
                    scan_start=1, options_with_values=frozenset({"-subtable"}), subcommand="add"
                ),
                OptionTerminatorSpec(
                    scan_start=1, options_with_values=frozenset({"-subtable"}), subcommand="replace"
                ),
                OptionTerminatorSpec(
                    scan_start=1, options_with_values=frozenset({"-subtable"}), subcommand="lookup"
                ),
            ),
            event_requires=EventRequires(flow=True),
            diagram_action=True,
            xc_translatable=False,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.SESSION_TABLE,
                    reads=True,
                    writes=True,
                    scope=StorageScope.SESSION_TABLE,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
            subcommands={
                sub: SubCommand(
                    name=sub,
                    arity=Arity(),
                    mutator=sub in _WRITE_SUBS,
                    side_effect_hints=(
                        SideEffect(
                            target=SideEffectTarget.SESSION_TABLE,
                            reads=sub in _READ_SUBS or sub in _WRITE_SUBS,
                            writes=sub in _WRITE_SUBS,
                            scope=StorageScope.SESSION_TABLE,
                            connection_side=ConnectionSide.BOTH,
                        ),
                    ),
                )
                for sub in (*_WRITE_SUBS, *_READ_SUBS)
            },
        )
