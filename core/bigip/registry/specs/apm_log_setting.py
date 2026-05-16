from __future__ import annotations

from ..models import (
    BigipObjectKindSpec,
    BigipObjectSpec,
    BigipPropertySpec,
)
from ._base import register


@register
def register_spec() -> BigipObjectSpec:
    return BigipObjectSpec(
        kind_spec=BigipObjectKindSpec(
            "apm_log_setting",
            module="apm",
            object_types=("log-setting",),
        ),
        header_types=(("apm", "log-setting"),),
        properties=(
            BigipPropertySpec(
                name="access",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="log-level",
                        value_type="unknown",
                        in_sections=("access",),
                        shape_kind="object",
                    ),
                    BigipPropertySpec(
                        name="publisher", value_type="string", in_sections=("access",)
                    ),
                ),
            ),
            BigipPropertySpec(
                name="log-level",
                value_type="unknown",
                in_sections=("access",),
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="access-control",
                        value_type="enum",
                        in_sections=("access", "log-level"),
                        enum_values=(
                            "alert",
                            "crit",
                            "debug",
                            "emerg",
                            "err",
                            "info",
                            "notice",
                            "warn",
                        ),
                    ),
                    BigipPropertySpec(
                        name="access-per-request",
                        value_type="enum",
                        in_sections=("access", "log-level"),
                        enum_values=(
                            "alert",
                            "crit",
                            "debug",
                            "emerg",
                            "err",
                            "info",
                            "notice",
                            "warn",
                        ),
                    ),
                    BigipPropertySpec(
                        name="apm-acl",
                        value_type="enum",
                        in_sections=("access", "log-level"),
                        enum_values=(
                            "alert",
                            "crit",
                            "debug",
                            "emerg",
                            "err",
                            "info",
                            "notice",
                            "warn",
                        ),
                    ),
                    BigipPropertySpec(
                        name="eca",
                        value_type="enum",
                        in_sections=("access", "log-level"),
                        enum_values=(
                            "alert",
                            "crit",
                            "debug",
                            "emerg",
                            "err",
                            "info",
                            "notice",
                            "warn",
                        ),
                    ),
                    BigipPropertySpec(
                        name="paa",
                        value_type="enum",
                        in_sections=("access", "log-level"),
                        enum_values=(
                            "alert",
                            "crit",
                            "debug",
                            "emerg",
                            "err",
                            "info",
                            "notice",
                            "warn",
                        ),
                    ),
                    BigipPropertySpec(
                        name="sso",
                        value_type="enum",
                        in_sections=("access", "log-level"),
                        enum_values=(
                            "alert",
                            "crit",
                            "debug",
                            "emerg",
                            "err",
                            "info",
                            "notice",
                            "warn",
                        ),
                    ),
                    BigipPropertySpec(
                        name="swg",
                        value_type="enum",
                        in_sections=("access", "log-level"),
                        enum_values=(
                            "alert",
                            "crit",
                            "debug",
                            "emerg",
                            "err",
                            "info",
                            "notice",
                            "warn",
                        ),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="access-control",
                value_type="enum",
                in_sections=("access", "log-level"),
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warn"),
            ),
            BigipPropertySpec(
                name="access-per-request",
                value_type="enum",
                in_sections=("access", "log-level"),
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warn"),
            ),
            BigipPropertySpec(
                name="apm-acl",
                value_type="enum",
                in_sections=("access", "log-level"),
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warn"),
            ),
            BigipPropertySpec(
                name="eca",
                value_type="enum",
                in_sections=("access", "log-level"),
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warn"),
            ),
            BigipPropertySpec(
                name="paa",
                value_type="enum",
                in_sections=("access", "log-level"),
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warn"),
            ),
            BigipPropertySpec(
                name="sso",
                value_type="enum",
                in_sections=("access", "log-level"),
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warn"),
            ),
            BigipPropertySpec(
                name="swg",
                value_type="enum",
                in_sections=("access", "log-level"),
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warn"),
            ),
            BigipPropertySpec(name="publisher", value_type="string", in_sections=("access",)),
            BigipPropertySpec(name="description", value_type="unknown"),
            BigipPropertySpec(
                name="url-filters",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="filter",
                        value_type="unknown",
                        in_sections=("url-filters",),
                        enum_values=("false", "true"),
                        shape_kind="object",
                    ),
                    BigipPropertySpec(
                        name="publisher",
                        value_type="string",
                        in_sections=("url-filters",),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="filter",
                value_type="unknown",
                in_sections=("url-filters",),
                enum_values=("false", "true"),
                shape_kind="object",
            ),
            BigipPropertySpec(name="publisher", value_type="string", in_sections=("url-filters",)),
        ),
    )
