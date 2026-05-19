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
            "apm_resource_remote_desktop_quest",
            module="apm",
            object_types=("resource remote-desktop quest",),
        ),
        header_types=(("apm", "resource remote-desktop quest"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="auto-logon",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="customization-group",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                default="none",
                block=(
                    BigipPropertySpec(
                        name="caption",
                        value_type="string",
                        in_sections=("customization-group",),
                        allow_none=True,
                    ),
                    BigipPropertySpec(
                        name="detailed-description",
                        value_type="string",
                        in_sections=("customization-group",),
                        allow_none=True,
                    ),
                ),
            ),
            BigipPropertySpec(
                name="caption",
                value_type="string",
                in_sections=("customization-group",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="detailed-description",
                value_type="string",
                in_sections=("customization-group",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="description",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="domain-source",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "session.logon.last.domain"),
                default="session",
            ),
            BigipPropertySpec(
                name="enable-serverside-ssl",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(name="host", value_type="unknown"),
            BigipPropertySpec(name="ip", value_type="string", shape_kind="ip-address"),
            BigipPropertySpec(
                name="location-specific",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="password-source",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "session.logon.last.password"),
                default="session",
            ),
            BigipPropertySpec(
                name="pool",
                value_type="reference",
                references=(
                    "analytics_lsn_pool_report",
                    "analytics_lsn_pool_scheduled_report",
                    "analytics_pool_traffic_report",
                    "analytics_pool_traffic_scheduled_report",
                    "gtm_pool_a",
                    "gtm_pool_aaaa",
                    "gtm_pool_cname",
                    "gtm_pool_https",
                    "gtm_pool_mx",
                    "gtm_pool_naptr",
                    "gtm_pool_srv",
                    "gtm_pool_svcb",
                    "gtm_prober_pool",
                    "ltm_lsn_pool",
                    "ltm_pool",
                ),
            ),
            BigipPropertySpec(name="port", value_type="string", allow_none=True, default="8080"),
            BigipPropertySpec(
                name="username-source",
                value_type="enum",
                allow_none=True,
                enum_values=("none",),
                default="session",
            ),
        ),
    )
