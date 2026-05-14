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
            "apm_saml_attribute_consuming_service",
            module="apm",
            object_types=("saml attribute-consuming-service",),
        ),
        header_types=(("apm", "saml attribute-consuming-service"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="attributes",
                value_type="list",
                required=True,
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="app-service",
                        value_type="string",
                        in_sections=("attributes",),
                        allow_none=True,
                        default="none",
                    ),
                    BigipPropertySpec(
                        name="attribute-name",
                        value_type="string",
                        in_sections=("attributes",),
                    ),
                    BigipPropertySpec(
                        name="friendly-name",
                        value_type="string",
                        in_sections=("attributes",),
                    ),
                    BigipPropertySpec(
                        name="is-required",
                        value_type="unknown",
                        in_sections=("attributes",),
                    ),
                    BigipPropertySpec(
                        name="name-format",
                        value_type="string",
                        in_sections=("attributes",),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                in_sections=("attributes",),
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="attribute-name",
                value_type="string",
                in_sections=("attributes",),
            ),
            BigipPropertySpec(
                name="friendly-name",
                value_type="string",
                in_sections=("attributes",),
            ),
            BigipPropertySpec(
                name="is-required",
                value_type="unknown",
                in_sections=("attributes",),
            ),
            BigipPropertySpec(name="name-format", value_type="string", in_sections=("attributes",)),
            BigipPropertySpec(name="service-description", value_type="string", allow_none=True),
            BigipPropertySpec(name="service-name", value_type="string", allow_none=True),
        ),
    )
