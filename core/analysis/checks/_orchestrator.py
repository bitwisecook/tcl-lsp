"""Check registry and runner."""

from __future__ import annotations

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
    """Run all registered checks on a single command and return diagnostics."""
    diagnostics: list[Diagnostic] = []
    for check in ALL_CHECKS:
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
