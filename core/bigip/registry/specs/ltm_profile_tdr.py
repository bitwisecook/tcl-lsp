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
            "ltm_profile_tdr",
            module="ltm",
            object_types=("profile tdr",),
        ),
        header_types=(("ltm", "profile tdr"),),
        properties=(
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_tdr",),
            ),
            BigipPropertySpec(name="description", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="filters",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="description", value_type="boolean", in_sections=("filters",), allow_none=True
            ),
            BigipPropertySpec(
                name="message-type",
                value_type="enum",
                in_sections=("filters",),
                enum_values=("all", "answer", "request"),
            ),
            BigipPropertySpec(
                name="tdr-format", value_type="boolean", in_sections=("filters",), allow_none=True
            ),
            BigipPropertySpec(
                name="traffic-direction",
                value_type="enum",
                in_sections=("filters",),
                enum_values=("all", "egress", "ingress"),
            ),
            BigipPropertySpec(
                name="condition-pattern-1", value_type="string", in_sections=("filters",)
            ),
            BigipPropertySpec(
                name="cmp-operator",
                value_type="enum",
                in_sections=("condition-pattern-1",),
                allow_none=True,
                enum_values=("contains", "ends-with", "equal", "none", "not-equal", "starts-with"),
            ),
            BigipPropertySpec(
                name="field-name",
                value_type="boolean",
                in_sections=("condition-pattern-1",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="field-value",
                value_type="boolean",
                in_sections=("condition-pattern-1",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="condition-pattern-2", value_type="string", in_sections=("filters",)
            ),
            BigipPropertySpec(
                name="cmp-operator",
                value_type="enum",
                in_sections=("condition-pattern-2",),
                allow_none=True,
                enum_values=("contains", "ends-with", "equal", "none", "not-equal", "starts-with"),
            ),
            BigipPropertySpec(
                name="field-name",
                value_type="boolean",
                in_sections=("condition-pattern-2",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="field-value",
                value_type="boolean",
                in_sections=("condition-pattern-2",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="condition-pattern-3", value_type="string", in_sections=("filters",)
            ),
            BigipPropertySpec(
                name="cmp-operator",
                value_type="enum",
                in_sections=("condition-pattern-3",),
                allow_none=True,
                enum_values=("contains", "ends-with", "equal", "none", "not-equal", "starts-with"),
            ),
            BigipPropertySpec(
                name="field-name",
                value_type="boolean",
                in_sections=("condition-pattern-3",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="field-value",
                value_type="boolean",
                in_sections=("condition-pattern-3",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="condition-pattern-4", value_type="string", in_sections=("filters",)
            ),
            BigipPropertySpec(
                name="cmp-operator",
                value_type="enum",
                in_sections=("condition-pattern-4",),
                allow_none=True,
                enum_values=("contains", "ends-with", "equal", "none", "not-equal", "starts-with"),
            ),
            BigipPropertySpec(
                name="field-name",
                value_type="boolean",
                in_sections=("condition-pattern-4",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="field-value",
                value_type="boolean",
                in_sections=("condition-pattern-4",),
                allow_none=True,
            ),
            BigipPropertySpec(name="log-publisher", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
