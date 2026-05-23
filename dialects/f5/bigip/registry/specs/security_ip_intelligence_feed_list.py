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
            "security_ip_intelligence_feed_list",
            module="security",
            object_types=("ip-intelligence feed-list",),
        ),
        header_types=(("security", "ip-intelligence feed-list"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="reference", default="none"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="feeds",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="default-blacklist-category",
                        value_type="string",
                        in_sections=("feeds",),
                    ),
                    BigipPropertySpec(
                        name="default-list-type",
                        value_type="enum",
                        in_sections=("feeds",),
                        enum_values=("blacklist", "whitelist"),
                    ),
                    BigipPropertySpec(
                        name="poll",
                        value_type="unknown",
                        in_sections=("feeds",),
                        shape_kind="object",
                    ),
                ),
            ),
            BigipPropertySpec(
                name="default-blacklist-category",
                value_type="string",
                in_sections=("feeds",),
            ),
            BigipPropertySpec(
                name="default-list-type",
                value_type="enum",
                in_sections=("feeds",),
                enum_values=("blacklist", "whitelist"),
            ),
            BigipPropertySpec(
                name="poll",
                value_type="unknown",
                in_sections=("feeds",),
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="interval",
                        value_type="integer",
                        in_sections=("feeds", "poll"),
                    ),
                    BigipPropertySpec(
                        name="password",
                        value_type="string",
                        in_sections=("feeds", "poll"),
                    ),
                    BigipPropertySpec(
                        name="url", value_type="string", in_sections=("feeds", "poll")
                    ),
                    BigipPropertySpec(
                        name="user", value_type="string", in_sections=("feeds", "poll")
                    ),
                ),
            ),
            BigipPropertySpec(name="interval", value_type="integer", in_sections=("feeds", "poll")),
            BigipPropertySpec(name="password", value_type="string", in_sections=("feeds", "poll")),
            BigipPropertySpec(name="url", value_type="string", in_sections=("feeds", "poll")),
            BigipPropertySpec(name="user", value_type="string", in_sections=("feeds", "poll")),
            BigipPropertySpec(
                name="load",
                value_type="unknown",
                references=("gtm_global_settings_load_balancing", "load"),
                shape_kind="object",
            ),
        ),
    )
