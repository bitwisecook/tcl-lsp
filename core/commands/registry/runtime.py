"""Registry runtime: dialect profile + role-aware signature resolution.

Command inventory, hover/completion metadata, and baseline arity come from the
registry command specs. This module owns runtime profile state and arg-role
semantics needed by analysis/formatting/compiler passes.
"""

from __future__ import annotations

from collections.abc import Iterator
from dataclasses import dataclass
from functools import lru_cache

from ...parsing.tokens import Token
from .command_registry import REGISTRY
from .models import CommandSpec, ValidationSpec
from .signatures import ArgRole, Arity, CommandSig, SubcommandSig
from .taint_hints import TaintHint
from .type_hints import CommandTypeHint, SubcommandTypeHint

# Re-export so existing callers keep working via ``from ...runtime import ...``.
__all__ = [
    "ArgRole",
    "BodyArgument",
    "CommandSig",
    "SubcommandSig",
    "CommandTypeHint",
    "SubcommandTypeHint",
    "TaintHint",
    "options_with_value",
    "regexp_pattern_index",
    "skip_options",
]


def _role_hints_from_registry() -> dict[str, CommandSig | SubcommandSig]:
    """Build role hints from inline arg_roles on CommandSpec/SubCommand.

    This replaces the old tcl_role_hints() aggregator.  The returned dict
    is used by _with_roles() to merge arg_roles onto validation-derived
    signatures for commands that haven't been fully migrated to SubCommand
    dicts (e.g. iRules overrides of Tcl commands).
    """
    hints: dict[str, CommandSig | SubcommandSig] = {}
    for name, specs in REGISTRY.specs_by_name.items():
        for spec in specs:
            if spec.subcommands:
                sub_hints = {}
                for sub_name, sub in spec.subcommands.items():
                    if sub.arg_roles:
                        sub_hints[sub_name] = CommandSig(
                            arity=sub.arity,
                            arg_roles=dict(sub.arg_roles),
                        )
                    else:
                        sub_hints[sub_name] = CommandSig(arity=sub.arity)
                if sub_hints:
                    hints.setdefault(
                        name,
                        SubcommandSig(
                            subcommands=sub_hints,
                            allow_unknown=spec.allow_unknown_subcommands,
                        ),
                    )
            elif spec.arg_roles:
                arity = spec.validation.arity if spec.validation else Arity()
                hints.setdefault(name, CommandSig(arity=arity, arg_roles=dict(spec.arg_roles)))
    return hints


_ROLE_HINTS: dict[str, CommandSig | SubcommandSig] = _role_hints_from_registry()


def _type_hints_from_registry() -> dict[str, CommandTypeHint | SubcommandTypeHint]:
    """Build TYPE_HINTS from inline return_type/arg_types on CommandSpec/SubCommand."""

    hints: dict[str, CommandTypeHint | SubcommandTypeHint] = {}
    for name, specs in REGISTRY.specs_by_name.items():
        for spec in specs:
            if spec.subcommands:
                sub_hints = {}
                for sub_name, sub in spec.subcommands.items():
                    if sub.return_type is not None or sub.arg_types:
                        sub_hints[sub_name] = CommandTypeHint(
                            return_type=sub.return_type,
                            arg_types=dict(sub.arg_types) if sub.arg_types else {},
                        )
                if sub_hints:
                    hints[name] = SubcommandTypeHint(subcommands=sub_hints)
            elif spec.return_type is not None or spec.arg_types:
                hints[name] = CommandTypeHint(
                    return_type=spec.return_type,
                    arg_types=dict(spec.arg_types) if spec.arg_types else {},
                )
    return hints


# Type hints derived from inline fields on CommandSpec/SubCommand.
TYPE_HINTS: dict[str, CommandTypeHint | SubcommandTypeHint] = _type_hints_from_registry()

# Taint hints from class-per-command definitions.
from .irules import irules_taint_hints as _irules_taint_hints  # noqa: E402
from .tcl import tcl_taint_hints as _tcl_taint_hints  # noqa: E402

TAINT_HINTS: dict[str, TaintHint] = {**_tcl_taint_hints(), **_irules_taint_hints()}

# Generic EDA-tooling profile seed. Kept intentionally conservative; users can
# extend with tclLsp.extraCommands.
_EDA_TOOLS_SIGNATURES: dict[str, CommandSig | SubcommandSig] = {
    "define_proc_attributes": CommandSig(arity=Arity(1)),
    "get_cells": CommandSig(),
    "get_pins": CommandSig(),
    "get_nets": CommandSig(),
    "all_registers": CommandSig(),
    "report_timing": CommandSig(),
    "create_clock": CommandSig(arity=Arity(1)),
    "set_false_path": CommandSig(),
    "set_multicycle_path": CommandSig(),
}

from .dialects import KNOWN_DIALECTS as _KNOWN_DIALECTS  # noqa: E402

_active_dialect = "tcl8.6"
_active_extra_commands: tuple[str, ...] = ()
SIGNATURES: dict[str, CommandSig | SubcommandSig] = {}


def _canonical_dialect(dialect: str) -> str | None:
    value = dialect.strip().lower()
    if value in _KNOWN_DIALECTS:
        return value
    return None


def _extra_command_signatures(extra_commands: list[str] | tuple[str, ...]) -> dict[str, CommandSig]:
    extra: dict[str, CommandSig] = {}
    for name in extra_commands:
        cmd = name.strip()
        if not cmd:
            continue
        extra[cmd] = CommandSig()
    return extra


def _with_roles(name: str, sig: CommandSig | SubcommandSig) -> CommandSig | SubcommandSig:
    """Merge role hints onto a validation-derived signature."""
    hint = _ROLE_HINTS.get(name)

    if isinstance(sig, CommandSig):
        if isinstance(hint, CommandSig):
            return CommandSig(
                arity=sig.arity,
                arg_roles=dict(hint.arg_roles),
            )
        return sig

    if not isinstance(sig, SubcommandSig):
        return sig

    hint_subs: dict[str, CommandSig] = {}
    if isinstance(hint, SubcommandSig):
        hint_subs = hint.subcommands

    merged_subs: dict[str, CommandSig] = {}
    for sub_name, sub_sig in sig.subcommands.items():
        sub_hint = hint_subs.get(sub_name)
        if isinstance(sub_hint, CommandSig):
            merged_subs[sub_name] = CommandSig(
                arity=sub_sig.arity,
                arg_roles=dict(sub_hint.arg_roles),
            )
        else:
            merged_subs[sub_name] = sub_sig

    return SubcommandSig(
        subcommands=merged_subs,
        allow_unknown=sig.allow_unknown,
    )


def _signature_from_validation(validation: ValidationSpec | None) -> CommandSig:
    if validation is None:
        return CommandSig()
    return CommandSig(arity=validation.arity)


def _signature_from_spec(spec: "CommandSpec") -> CommandSig | SubcommandSig:
    """Derive a signature from a CommandSpec, preferring SubCommand data.

    For commands with subcommands: reads arg_roles from SubCommand objects.
    For simple commands: reads arg_roles from CommandSpec.arg_roles,
    falling back to _signature_from_validation when empty.
    """
    if spec.subcommands:
        return SubcommandSig(
            subcommands={
                sub_name: CommandSig(
                    arity=sub.arity,
                    arg_roles=dict(sub.arg_roles) if sub.arg_roles else {},
                )
                for sub_name, sub in spec.subcommands.items()
            },
            allow_unknown=spec.allow_unknown_subcommands,
        )
    # Simple command: use inline arg_roles if available.
    if spec.arg_roles or spec.arg_role_resolver:
        arity = spec.validation.arity if spec.validation else Arity()
        return CommandSig(
            arity=arity,
            arg_roles=dict(spec.arg_roles) if spec.arg_roles else {},
            arg_role_resolver=spec.arg_role_resolver,
        )
    return _signature_from_validation(spec.validation)


def _registry_signatures_for_dialect(dialect: str) -> dict[str, CommandSig | SubcommandSig]:
    signatures: dict[str, CommandSig | SubcommandSig] = {}
    for name in REGISTRY.command_names(dialect):
        # Skip tcllib commands — they are handled separately.
        if REGISTRY.is_tcllib_command(name):
            continue
        spec = REGISTRY.get(name, dialect)
        if spec is not None:
            sig = _signature_from_spec(spec)
        else:
            sig = _signature_from_validation(REGISTRY.validation(name, dialect))
        signatures[name] = _with_roles(name, sig)
    return signatures


def _registry_signatures_for_tcllib() -> dict[str, CommandSig | SubcommandSig]:
    """Build signatures for all tcllib commands from registry specs."""
    signatures: dict[str, CommandSig | SubcommandSig] = {}
    for name in REGISTRY.all_tcllib_command_names():
        spec = REGISTRY.get_any(name)
        if spec is not None:
            sig = _signature_from_spec(spec)
        else:
            sig = _signature_from_validation(REGISTRY.validation(name))
        signatures[name] = _with_roles(name, sig)
    return signatures


def _build_signatures(
    dialect: str,
    *,
    extra_commands: list[str] | tuple[str, ...],
) -> dict[str, CommandSig | SubcommandSig]:
    signatures: dict[str, CommandSig | SubcommandSig]

    match dialect:
        case "tcl8.4" | "tcl8.5" | "tcl8.6" | "tcl9.0":
            signatures = _registry_signatures_for_dialect(dialect)
        case "f5-irules":
            signatures = _registry_signatures_for_dialect("tcl8.6")
            signatures.update(_registry_signatures_for_dialect("f5-irules"))
        case "f5-iapps":
            signatures = _registry_signatures_for_dialect("tcl8.6")
            signatures.update(_registry_signatures_for_dialect("f5-iapps"))
        case "eda-tools":
            signatures = _registry_signatures_for_dialect("tcl8.6")
            for name, sig in _EDA_TOOLS_SIGNATURES.items():
                signatures[name] = _with_roles(name, sig)
        case "expect":
            signatures = _registry_signatures_for_dialect("tcl8.6")
            signatures.update(_registry_signatures_for_dialect("expect"))
        case _:
            # Guard for defensive callers; configure_signatures should validate.
            return {}

    # Tcllib commands are always included in SIGNATURES because their
    # namespaced names (e.g. json::json2dict) don't collide with core Tcl.
    # Per-document filtering happens in the feature layer (completion,
    # hover, diagnostics) by checking ``package require`` statements.
    signatures.update(_registry_signatures_for_tcllib())

    signatures.update(_extra_command_signatures(extra_commands))
    return signatures


def available_dialects() -> list[str]:
    """Return canonical dialect profile names."""
    return sorted(_KNOWN_DIALECTS)


def active_signature_profile() -> dict[str, object]:
    """Return the currently active command-signature profile."""
    return {
        "dialect": _active_dialect,
        "extra_commands": list(_active_extra_commands),
    }


def is_irules_dialect() -> bool:
    """Return True if the active dialect is iRules."""
    return _active_dialect == "f5-irules"


def configure_signatures(
    *,
    dialect: str | None = None,
    extra_commands: list[str] | tuple[str, ...] | None = None,
) -> bool:
    """Configure active command signatures.

    Returns ``True`` if the effective profile changed.
    """
    global _active_dialect, _active_extra_commands

    if dialect is None:
        next_dialect = _active_dialect
    else:
        requested = _canonical_dialect(dialect)
        if requested is None:
            return False
        next_dialect = requested
    if extra_commands is None:
        next_extra = _active_extra_commands
    else:
        next_extra = tuple(
            sorted({name.strip() for name in extra_commands if name and name.strip()})
        )

    if next_dialect == _active_dialect and next_extra == _active_extra_commands and SIGNATURES:
        return False

    new_signatures = _build_signatures(
        next_dialect,
        extra_commands=next_extra,
    )
    SIGNATURES.clear()
    SIGNATURES.update(new_signatures)
    _active_dialect = next_dialect
    _active_extra_commands = next_extra

    # Configure lexer flags for the active dialect.
    from core.parsing.lexer import TclLexer

    TclLexer.irules_brace_separator = next_dialect == "f5-irules"

    return True


def _derive_oo_define_subcommands() -> frozenset[str]:
    """Derive OO definition subcommand names from the oo::define registry spec."""
    spec = REGISTRY.get("oo::define")
    if spec is None:
        return frozenset()
    return frozenset(v.value for vals in spec.forms for v in vals.arg_values.get(0, ()))


_TCL_OO_DEFINE_SUBCOMMANDS: frozenset[str] = frozenset()  # populated after init


def _oo_class_object_body_indices(args: list[str]) -> set[int]:
    """Return BODY argument indices for ``oo::class`` / ``oo::object``."""
    if len(args) < 2:
        return set()
    subcommand = args[0]
    if subcommand == "create" and len(args) >= 3:
        return {2}
    if subcommand == "new" and len(args) >= 2:
        return {1}
    return set()


def _oo_define_body_indices(args: list[str]) -> set[int]:
    """Return BODY argument indices for ``oo::define`` / ``oo::objdefine``."""
    # Script form: oo::define Target { ...definition script... }
    if len(args) == 2 and args[1] not in _TCL_OO_DEFINE_SUBCOMMANDS:
        return {1}

    if len(args) < 2:
        return set()

    subcommand = args[1]
    if subcommand == "constructor" and len(args) >= 4:
        return {3}
    if subcommand == "destructor" and len(args) >= 3:
        return {2}
    if subcommand == "method" and len(args) >= 5:
        return {4}
    if subcommand == "self" and len(args) >= 3:
        self_subcommand = args[2]
        if self_subcommand == "constructor" and len(args) >= 5:
            return {4}
        if self_subcommand == "destructor" and len(args) >= 4:
            return {3}
        if self_subcommand == "method" and len(args) >= 6:
            return {5}
    return set()


def _oo_definition_body_indices(command: str, args: list[str]) -> set[int]:
    """Return BODY argument indices for TclOO definition-script commands."""
    if command == "constructor" and len(args) >= 2:
        return {1}
    if command == "destructor" and len(args) >= 1:
        return {0}
    if command == "method" and len(args) >= 3:
        return {2}
    if command == "self" and args:
        subcommand = args[0]
        if subcommand == "constructor" and len(args) >= 3:
            return {2}
        if subcommand == "destructor" and len(args) >= 2:
            return {1}
        if subcommand == "method" and len(args) >= 4:
            return {3}
    return set()


def _if_body_indices(args: list[str]) -> set[int]:
    """Return BODY argument indices for an ``if`` command."""
    result: set[int] = set()
    i = 0

    # Initial: if expr ?then? body
    if i < len(args):
        i += 1  # expr
    if i < len(args) and args[i] == "then":
        i += 1
    if i < len(args):
        result.add(i)
        i += 1

    # Repeated: ?elseif expr ?then? body? ... ?else body?
    while i < len(args):
        kw = args[i]
        if kw == "elseif":
            i += 1  # keyword
            if i < len(args):
                i += 1  # expr
            if i < len(args) and args[i] == "then":
                i += 1
            if i < len(args):
                result.add(i)
                i += 1
            continue
        if kw == "else":
            if i + 1 < len(args):
                result.add(i + 1)
            break
        i += 1

    return result


def _if_expr_indices(args: list[str]) -> set[int]:
    """Return EXPR argument indices for an ``if`` command."""
    result: set[int] = set()
    i = 0

    # Initial: if expr ?then? body
    if i < len(args):
        result.add(i)
        i += 1
    if i < len(args) and args[i] == "then":
        i += 1
    if i < len(args):
        i += 1  # body

    # Repeated: ?elseif expr ?then? body? ... ?else body?
    while i < len(args):
        kw = args[i]
        if kw == "elseif":
            i += 1  # keyword
            if i < len(args):
                result.add(i)
                i += 1
            if i < len(args) and args[i] == "then":
                i += 1
            if i < len(args):
                i += 1  # body
            continue
        if kw == "else":
            break
        i += 1

    return result


def _try_body_indices(args: list[str]) -> set[int]:
    """Return BODY argument indices for a ``try`` command."""
    result: set[int] = set()
    if args:
        result.add(0)  # try body

    i = 1
    while i < len(args):
        kw = args[i]
        if kw == "finally":
            if i + 1 < len(args):
                result.add(i + 1)
            i += 2
        elif kw in ("on", "trap"):
            # on code varList body  /  trap pattern varList body
            if i + 3 < len(args):
                result.add(i + 3)
            i += 4
        else:
            i += 1
    return result


def _switch_body_indices(args: list[str]) -> set[int]:
    """Return BODY argument indices for a ``switch`` command."""
    result: set[int] = set()
    i = 0

    # Skip option flags. '--' terminates option parsing.
    while i < len(args) and args[i].startswith("-"):
        if args[i] == "--":
            i += 1
            break
        i += 1

    # Skip switch string argument.
    if i >= len(args):
        return result
    i += 1
    if i >= len(args):
        return result

    # Braced list form: single trailing argument.
    if i == len(args) - 1:
        result.add(i)
        return result

    # List form: pattern body pattern body ...
    while i + 1 < len(args):
        if args[i + 1] != "-":
            result.add(i + 1)
        i += 2

    return result


def regexp_pattern_index(args: list[str] | tuple[str, ...]) -> int | None:
    """Return the pattern argument index for regexp/regsub (0-based after cmd).

    Skips over option switches (``-nocase``, ``-start N``, ``--``, etc.)
    to find the first positional argument which is the regex pattern.

    *args* should **not** include the command name itself.
    """
    i = skip_options(args, options_with_value("regexp"))
    if i < len(args):
        return i
    return None


@lru_cache(maxsize=None)
def options_with_value(command: str) -> frozenset[str]:
    """Return the set of option names that consume a following value argument.

    Derived from ``OptionSpec.takes_value`` on the command's first form.
    The result is cached since the registry is immutable after initialisation.
    """
    spec = REGISTRY.get_any(command)
    if spec is None or not spec.forms:
        return frozenset()
    return frozenset(opt.name for opt in spec.forms[0].options if opt.takes_value)


def skip_options(
    args: list[str] | tuple[str, ...],
    value_options: frozenset[str] | None = None,
) -> int:
    """Return the index of the first non-option argument.

    Scans *args* (0-based, command name excluded) skipping ``-option`` flags
    and their values.  *value_options* is the set of options that consume a
    following value argument; if ``None``, only the ``--`` terminator is
    recognised.
    """
    if value_options is None:
        value_options = frozenset()
    i = 0
    while i < len(args):
        arg = args[i]
        if arg == "--":
            i += 1
            break
        if arg.startswith("-"):
            i += 1
            if arg in value_options and i < len(args):
                i += 1
            continue
        break
    return i


def arg_indices_for_role(command: str, args: list[str], role: ArgRole) -> set[int]:
    """Return argument indices (0-based, after command name) for a role."""
    if role is ArgRole.BODY:
        if command in ("oo::class", "oo::object"):
            return _oo_class_object_body_indices(args)
        if command in ("oo::define", "oo::objdefine"):
            return _oo_define_body_indices(args)
        oo_body = _oo_definition_body_indices(command, args)
        if oo_body:
            return oo_body
        if command == "when" and len(args) >= 2:
            return {len(args) - 1}
        if command == "if":
            return _if_body_indices(args)
        if command == "try":
            return _try_body_indices(args)
        if command == "switch":
            return _switch_body_indices(args)
        if command == "foreach" and len(args) >= 3:
            return {len(args) - 1}
        if command == "lmap" and len(args) >= 3:
            return {len(args) - 1}
    if role is ArgRole.EXPR:
        if command == "if":
            return _if_expr_indices(args)
    if role is ArgRole.PATTERN:
        if command in ("regexp", "regsub"):
            idx = regexp_pattern_index(args)
            if idx is not None:
                return {idx}
            return set()

    sig = SIGNATURES.get(command)
    if sig is None:
        return set()

    if isinstance(sig, SubcommandSig):
        if not args:
            return set()
        sub_sig = sig.subcommands.get(args[0])
        if sub_sig is None:
            return set()
        return {
            idx + 1
            for idx, arg_role in sub_sig.arg_roles.items()
            if arg_role is role and (idx + 1) < len(args)
        }

    if isinstance(sig, CommandSig):
        # Dynamic resolver takes priority for variable-layout commands.
        if sig.arg_role_resolver is not None:
            resolved = sig.arg_role_resolver(args)
            return {idx for idx, r in resolved.items() if r is role and idx < len(args)}
        result = {
            idx for idx, arg_role in sig.arg_roles.items() if arg_role is role and idx < len(args)
        }
        if role is ArgRole.BODY and command == "foreach" and len(args) >= 3:
            result.add(len(args) - 1)
        return result

    return set()


def body_arg_indices(command: str, args: list[str]) -> set[int]:
    """Return BODY argument indices for *command* given args after command name."""
    return arg_indices_for_role(command, args, ArgRole.BODY)


def expr_arg_indices(command: str, args: list[str]) -> set[int]:
    """Return EXPR argument indices for *command* given args after command name."""
    return arg_indices_for_role(command, args, ArgRole.EXPR)


# Body argument iteration


@dataclass(frozen=True, slots=True)
class BodyArgument:
    """A validated body argument from a Tcl command.

    Yielded by :func:`iter_body_arguments` after bounds-checking.
    """

    index: int
    """0-based argument index (after command name)."""

    text: str
    """The body text (``args[index]``)."""

    token: Token
    """The token for this argument (``arg_tokens[index]``)."""


def iter_body_arguments(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
) -> Iterator[BodyArgument]:
    """Yield validated :class:`BodyArgument` entries for *cmd_name*.

    Resolves ``ArgRole.BODY`` indices via :func:`arg_indices_for_role` and
    yields one ``BodyArgument`` per index that is within bounds of both
    *args* and *arg_tokens*.  Indices are yielded in ascending order.

    Callers should apply any further filtering they need (e.g. checking
    ``body.token.type is TokenType.STR`` or ``body.text.strip()``).
    """
    for idx in sorted(arg_indices_for_role(cmd_name, args, ArgRole.BODY)):
        if idx >= len(args) or idx >= len(arg_tokens):
            continue
        yield BodyArgument(index=idx, text=args[idx], token=arg_tokens[idx])


# Initialize runtime signatures for default profile.
configure_signatures(dialect="tcl8.6", extra_commands=[])
_TCL_OO_DEFINE_SUBCOMMANDS = _derive_oo_define_subcommands()
