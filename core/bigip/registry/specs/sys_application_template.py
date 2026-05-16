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
            "sys_application_template",
            module="sys",
            object_types=("application template",),
        ),
        header_types=(("sys", "application template"),),
        properties=(
            BigipPropertySpec(
                name="actions",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="definition",
                        value_type="unknown",
                        in_sections=("actions",),
                        shape_kind="object",
                    ),
                ),
            ),
            BigipPropertySpec(
                name="definition",
                value_type="unknown",
                in_sections=("actions",),
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="html-help",
                        value_type="string",
                        in_sections=("actions", "definition"),
                    ),
                    BigipPropertySpec(
                        name="implementation",
                        value_type="unknown",
                        in_sections=("actions", "definition"),
                        shape_kind="object",
                    ),
                    BigipPropertySpec(
                        name="presentation",
                        value_type="unknown",
                        in_sections=("actions", "definition"),
                        shape_kind="object",
                    ),
                    BigipPropertySpec(
                        name="role-acl",
                        value_type="list",
                        in_sections=("actions", "definition"),
                        allow_none=True,
                        list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                    ),
                    BigipPropertySpec(
                        name="run-as",
                        value_type="string",
                        in_sections=("actions", "definition"),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="html-help",
                value_type="string",
                in_sections=("actions", "definition"),
            ),
            BigipPropertySpec(
                name="implementation",
                value_type="unknown",
                in_sections=("actions", "definition"),
                shape_kind="object",
            ),
            BigipPropertySpec(
                name="presentation",
                value_type="unknown",
                in_sections=("actions", "definition"),
                shape_kind="object",
            ),
            BigipPropertySpec(
                name="role-acl",
                value_type="list",
                in_sections=("actions", "definition"),
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="run-as",
                value_type="string",
                in_sections=("actions", "definition"),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="metadata",
                value_type="unknown",
                default="persistent, which saves the data into the config file",
            ),
            BigipPropertySpec(
                name="persist",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="requires-bigip-version-max",
                value_type="string",
                required=True,
            ),
            BigipPropertySpec(
                name="requires-bigip-version-min",
                value_type="string",
                required=True,
            ),
            BigipPropertySpec(
                name="requires-modules",
                value_type="list",
                required=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(name="value", value_type="string"),
            BigipPropertySpec(
                name="ignore-verification",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="signing-key",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="tmpl-checksum",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="tmpl-signature",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
        ),
    )
