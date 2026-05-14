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
            "apm_sso_form_basedv2",
            module="apm",
            object_types=("sso form-basedv2",),
        ),
        header_types=(("apm", "sso form-basedv2"),),
        properties=(
            BigipPropertySpec(name="apm-log-config", value_type="string", allow_none=True),
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="forms",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                in_sections=("forms",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="attribute-value",
                value_type="string",
                in_sections=("forms",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="controls",
                value_type="list",
                in_sections=("forms",),
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                in_sections=("forms", "controls"),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="secure",
                value_type="enum",
                in_sections=("forms", "controls"),
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(name="value", value_type="string", in_sections=("forms", "controls")),
            BigipPropertySpec(
                name="description",
                value_type="string",
                in_sections=("forms",),
                allow_none=True,
            ),
            BigipPropertySpec(name="form-order", value_type="integer", in_sections=("forms",)),
            BigipPropertySpec(
                name="id-type",
                value_type="enum",
                in_sections=("forms",),
                enum_values=("action", "id", "inputs", "order"),
            ),
            BigipPropertySpec(
                name="request-method",
                value_type="enum",
                in_sections=("forms",),
                enum_values=("get", "post"),
            ),
            BigipPropertySpec(
                name="request-name",
                value_type="string",
                in_sections=("forms",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="request-negative",
                value_type="enum",
                in_sections=("forms",),
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="request-prefix",
                value_type="enum",
                in_sections=("forms",),
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="request-type",
                value_type="enum",
                in_sections=("forms",),
                enum_values=("cookie", "header", "uri"),
            ),
            BigipPropertySpec(
                name="request-value",
                value_type="string",
                in_sections=("forms",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="submit-autodetect",
                value_type="enum",
                in_sections=("forms",),
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="submit-javascript",
                value_type="string",
                in_sections=("forms",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="submit-javascript-type",
                value_type="enum",
                in_sections=("forms",),
                enum_values=("auto", "custom", "extra"),
            ),
            BigipPropertySpec(name="submit-method", value_type="unknown", in_sections=("forms",)),
            BigipPropertySpec(
                name="submit-name",
                value_type="string",
                in_sections=("forms",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="submit-negative",
                value_type="enum",
                in_sections=("forms",),
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="submit-prefix",
                value_type="enum",
                in_sections=("forms",),
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="submit-type",
                value_type="enum",
                in_sections=("forms",),
                enum_values=("cookie", "header", "uri"),
            ),
            BigipPropertySpec(
                name="submit-value",
                value_type="string",
                in_sections=("forms",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="success-match-type",
                value_type="enum",
                in_sections=("forms",),
                allow_none=True,
                enum_values=("cookie", "none", "url"),
            ),
            BigipPropertySpec(
                name="success-match-value",
                value_type="string",
                in_sections=("forms",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="headers",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                in_sections=("headers",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="description",
                value_type="string",
                in_sections=("headers",),
                allow_none=True,
            ),
            BigipPropertySpec(name="value", value_type="string", in_sections=("headers",)),
            BigipPropertySpec(
                name="log-level",
                value_type="enum",
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warn"),
            ),
        ),
    )
