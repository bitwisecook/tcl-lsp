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
            BigipPropertySpec(
                name="feeds",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="name", value_type="string", in_sections=("feeds",)),
            BigipPropertySpec(
                name="default-blacklist-category", value_type="string", in_sections=("name",)
            ),
            BigipPropertySpec(
                name="default-list-type",
                value_type="enum",
                in_sections=("name",),
                enum_values=("whitelist", "blacklist"),
            ),
            BigipPropertySpec(name="poll", value_type="string", in_sections=("name",)),
            BigipPropertySpec(name="interval", value_type="integer", in_sections=("poll",)),
            BigipPropertySpec(
                name="user",
                value_type="reference",
                in_sections=("poll",),
                references=("auth_user",),
            ),
            BigipPropertySpec(name="url", value_type="string", in_sections=("poll",)),
            BigipPropertySpec(name="password", value_type="string", in_sections=("poll",)),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="load", value_type="list", repeated=True),
            BigipPropertySpec(name="security", value_type="string"),
            BigipPropertySpec(name="feeds", value_type="string", in_sections=("security",)),
            BigipPropertySpec(name="url2", value_type="string", in_sections=("feeds",)),
            BigipPropertySpec(name="poll", value_type="string", in_sections=("url2",)),
            BigipPropertySpec(name="description", value_type="string", in_sections=("feeds",)),
        ),
    )
