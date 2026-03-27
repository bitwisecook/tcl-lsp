"""Check registry and runner."""

from __future__ import annotations

from collections import defaultdict

from ...parsing.tokens import Token
from ..irules_checks import check_deprecated_event as check_deprecated_irules_event
from ..irules_checks import check_when_missing_priority
from ..semantic_model import Diagnostic
from ._domain import (
    check_deprecated_irules_command,
    check_disabled_command,
    check_irules_unnormalized_http_getter,
    check_literal_expected,
    check_non_literal_command,
    check_unknown_irules_event,
    check_unsafe_irules_command,
)
from ._security import (
    check_destructive_file_ops,
    check_eval_string_concat,
    check_eval_subst_double_decode,
    check_hardcoded_credentials,
    check_interp_eval_injection,
    check_open_pipeline,
    check_redos,
    check_source_variable,
    check_subst_injection,
    check_uplevel_injection,
)
from ._style import (
    check_binary_format_modifiers,
    check_encoding_mismatch,
    check_invalid_subnet_mask,
    check_loop_bound_inequality,
    check_missing_option_terminator,
    check_mistyped_ipv4,
    check_name_vs_value,
    check_non_ascii,
    check_path_concatenation,
    check_redundant_expr,
    check_string_compare_in_expr,
    check_string_list_confusion,
    check_subst_nocommands,
    check_unbraced_body,
    check_unbraced_expr,
    check_unbraced_switch_body,
)
from ._syntax import (
    check_unmatched_close_brace,
    check_unmatched_close_bracket,
)

# Type alias for check function signature.
_CheckFn = type(check_non_ascii)

# All checks that run on every command.  Each function has the signature:
#   (cmd_name, args, arg_tokens, all_tokens, source) -> list[Diagnostic]
ALL_CHECKS = [
    check_disabled_command,
    check_deprecated_irules_command,
    check_unsafe_irules_command,
    check_irules_unnormalized_http_getter,
    check_non_literal_command,
    check_unbraced_expr,
    check_eval_string_concat,
    check_subst_injection,
    check_subst_nocommands,
    check_eval_subst_double_decode,
    check_open_pipeline,
    check_string_list_confusion,
    check_unbraced_body,
    check_unbraced_switch_body,
    check_source_variable,
    check_uplevel_injection,
    check_redos,
    check_missing_option_terminator,
    check_unknown_irules_event,
    check_deprecated_irules_event,
    check_when_missing_priority,
    check_literal_expected,
    check_non_ascii,
    check_string_compare_in_expr,
    check_path_concatenation,
    check_binary_format_modifiers,
    check_name_vs_value,
    check_unmatched_close_bracket,
    check_unmatched_close_brace,
    check_loop_bound_inequality,
    check_redundant_expr,
    check_hardcoded_credentials,
    check_encoding_mismatch,
    check_interp_eval_injection,
    check_destructive_file_ops,
    check_invalid_subnet_mask,
    check_mistyped_ipv4,
]

# Checks that run unconditionally on every command (no command-name guard).
_UNIVERSAL_CHECKS: list[_CheckFn] = [
    check_disabled_command,
    check_non_literal_command,
    check_non_ascii,
    check_hardcoded_credentials,
    check_invalid_subnet_mask,
    check_mistyped_ipv4,
    check_unmatched_close_bracket,
    check_unmatched_close_brace,
    # Dialect-gated but not command-gated — run for all commands.
    check_deprecated_irules_command,
    check_unsafe_irules_command,
    # These use registry/role lookups and handle many commands.
    check_unbraced_expr,
    check_unbraced_body,
    check_missing_option_terminator,
    check_string_compare_in_expr,
    check_redundant_expr,
    check_name_vs_value,
]

# Checks that only apply to specific commands.  The dict maps command
# names to the list of additional checks to run beyond the universal set.
# This avoids calling ~20 check functions (each with an early-return guard)
# for commands that can never trigger them.
_TARGETED_CHECKS: dict[str, list[_CheckFn]] = defaultdict(list)

_CMD_CHECK_MAP: list[tuple[frozenset[str], _CheckFn]] = [
    (frozenset({"eval"}), check_eval_string_concat),
    (frozenset({"eval", "uplevel"}), check_eval_subst_double_decode),
    (frozenset({"subst"}), check_subst_injection),
    (frozenset({"subst"}), check_subst_nocommands),
    (frozenset({"open"}), check_open_pipeline),
    (frozenset({"append"}), check_string_list_confusion),
    (frozenset({"switch"}), check_unbraced_switch_body),
    (frozenset({"source"}), check_source_variable),
    (frozenset({"uplevel"}), check_uplevel_injection),
    (frozenset({"regexp", "regsub", "switch"}), check_redos),
    (frozenset({"when"}), check_unknown_irules_event),
    (frozenset({"when"}), check_deprecated_irules_event),
    (frozenset({"when"}), check_when_missing_priority),
    (frozenset({"regexp", "regsub", "class"}), check_literal_expected),
    (frozenset({"set"}), check_path_concatenation),
    (frozenset({"binary"}), check_binary_format_modifiers),
    (frozenset({"while"}), check_loop_bound_inequality),
    (frozenset({"fconfigure", "chan"}), check_encoding_mismatch),
    (frozenset({"interp"}), check_interp_eval_injection),
    # check_destructive_file_ops uses the registry which may match various
    # commands — keep it targeted to the known destructive command families.
    (frozenset({"file", "namespace", "chan"}), check_destructive_file_ops),
    # HTTP::path, HTTP::uri, HTTP::query — iRules commands
    (
        frozenset({"HTTP::path", "HTTP::uri", "HTTP::query"}),
        check_irules_unnormalized_http_getter,
    ),
]

for _cmds, _check in _CMD_CHECK_MAP:
    for _cmd in _cmds:
        _TARGETED_CHECKS[_cmd].append(_check)


def run_all_checks(
    cmd_name: str,
    args: list[str],
    arg_tokens: list[Token],
    all_tokens: list[Token],
    source: str,
    *,
    event: str | None = None,
    file_profiles: frozenset[str] = frozenset(),
) -> list[Diagnostic]:
    """Run all registered checks on a single command and return diagnostics.

    Uses command-name dispatch to skip checks that cannot apply to the
    current command, reducing the number of function calls from ~35 to
    ~8-12 per command on average.
    """
    diagnostics: list[Diagnostic] = []
    for check in _UNIVERSAL_CHECKS:
        diagnostics.extend(check(cmd_name, args, arg_tokens, all_tokens, source))
    targeted = _TARGETED_CHECKS.get(cmd_name)
    if targeted:
        for check in targeted:
            diagnostics.extend(check(cmd_name, args, arg_tokens, all_tokens, source))
    # iRules event-aware checks (requires event context from enclosing ``when``).
    if event is not None:
        from ..irules_checks import run_irules_event_checks

        diagnostics.extend(
            run_irules_event_checks(
                cmd_name,
                args,
                arg_tokens,
                all_tokens,
                source,
                event=event,
                file_profiles=file_profiles,
            )
        )
    return diagnostics
