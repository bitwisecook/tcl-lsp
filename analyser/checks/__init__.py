# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Tcl best-practice and safety checks.

Each check inspects a single parsed command (argv tokens + texts) and returns
zero or more Diagnostic objects.  The checks are called from the Analyser
during its normal single-pass walk.

Diagnostic code conventions:
    W1xx  -- style / best-practice warnings
    W2xx  -- potential bug warnings
    W3xx  -- security / injection warnings (W310–W313 added 2026-03)
"""

from __future__ import annotations

from ._bounds import (
    check_list_index_out_of_range,
    check_loop_termination,
    check_lset_index_out_of_range,
    check_string_index_out_of_range,
)
from ._domain import (
    check_deprecated_irules_command,
    check_dialect_invalid_expr_operator,
    check_dialect_invalid_option,
    check_disabled_command,
    check_irules_unnormalized_http_getter,
    check_literal_expected,
    check_non_literal_command,
    check_unknown_irules_event,
    check_unsafe_irules_command,
)
from ._orchestrator import (
    ALL_CHECKS,
    check_deprecated_irules_event,
    check_when_missing_priority,
    run_all_checks,
)
from ._security import (
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
    check_exec_not_captured,
    check_loop_bound_inequality,
    check_missing_option_terminator,
    check_name_vs_value,
    check_namespace_var_declaration,
    check_non_ascii,
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
