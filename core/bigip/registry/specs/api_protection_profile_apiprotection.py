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
            "api_protection_profile_apiprotection",
            module="api-protection",
            object_types=("profile apiprotection",),
        ),
        header_types=(("api-protection", "profile apiprotection"),),
        properties=(
            BigipPropertySpec(
                name="access-profile",
                value_type="unknown",
                default="none if created using TMSH",
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="default-response", value_type="unknown"),
            BigipPropertySpec(name="default-server", value_type="unknown", allow_none=True),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("api_protection_profile_apiprotection",),
                default="apiprotection",
            ),
            BigipPropertySpec(name="description", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="dns-mode",
                value_type="string",
                shape_kind="ip-address",
                default="ipv4-only",
            ),
            BigipPropertySpec(name="dns-resolver", value_type="unknown", allow_none=True),
            BigipPropertySpec(name="last-generated-path-id", value_type="integer"),
            BigipPropertySpec(
                name="max-concurrent-subsessions",
                value_type="integer",
                default="0, which sets the maximum number of concurrent subsessions to 5 times the licensed access session limit",
            ),
            BigipPropertySpec(name="openapi-version", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="paths",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="active",
                        value_type="enum",
                        in_sections=("paths",),
                        enum_values=("false", "true"),
                        shape_kind="boolean",
                        default="true",
                    ),
                    BigipPropertySpec(
                        name="app-service",
                        value_type="string",
                        in_sections=("paths",),
                        allow_none=True,
                        default="none",
                    ),
                    BigipPropertySpec(
                        name="description",
                        value_type="string",
                        in_sections=("paths",),
                        allow_none=True,
                    ),
                    BigipPropertySpec(name="method", value_type="string", in_sections=("paths",)),
                    BigipPropertySpec(name="path-id", value_type="integer", in_sections=("paths",)),
                    BigipPropertySpec(
                        name="server",
                        value_type="unknown",
                        in_sections=("paths",),
                        allow_none=True,
                    ),
                    BigipPropertySpec(name="uri", value_type="string", in_sections=("paths",)),
                ),
            ),
            BigipPropertySpec(
                name="active",
                value_type="enum",
                in_sections=("paths",),
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="true",
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                in_sections=("paths",),
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="description",
                value_type="string",
                in_sections=("paths",),
                allow_none=True,
            ),
            BigipPropertySpec(name="method", value_type="string", in_sections=("paths",)),
            BigipPropertySpec(name="path-id", value_type="integer", in_sections=("paths",)),
            BigipPropertySpec(
                name="server",
                value_type="unknown",
                in_sections=("paths",),
                allow_none=True,
            ),
            BigipPropertySpec(name="uri", value_type="string", in_sections=("paths",)),
            BigipPropertySpec(name="per-request-policy", value_type="unknown"),
            BigipPropertySpec(
                name="responses",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="servers",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="use-pool",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
        ),
    )
