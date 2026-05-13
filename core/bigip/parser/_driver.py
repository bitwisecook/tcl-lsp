"""Top-level driver: :func:`parse_bigip_conf`.

Reads dispatch tables from :mod:`._parsers` and routes each parsed
block to the right per-kind sub-parser.  This module IS the public
entry point of the parser package — :mod:`core.bigip.parser` just
re-exports :func:`parse_bigip_conf` from here.
"""

from __future__ import annotations

from ..model import BigipConfig, BigipGenericObject, DataGroupType
from ._helpers import (
    DocumentBuffer,
    _extract_blocks,
    _parse_generic_header,
    _range_from_offsets,
)
from ._parsers import *  # noqa: F401,F403 — make every per-kind parser + dispatch table available to ``parse_bigip_conf`` without enumerating 100+ underscore names.
from ._parsers import (  # noqa: F401 — explicit imports for the names the driver actually calls; ``import *`` skips underscore-prefixed names.
    _APM_MINIMAL_DISPATCH,
    _LTM_AUTH_DISPATCH,
    _LTM_MESSAGE_ROUTING_DISPATCH,
    _LTM_MINIMAL_DISPATCH,
    _MINIMAL_DISPATCH_BY_MODULE,
    _NAMED_DISPATCH,
    _PEM_MINIMAL_DISPATCH,
    _SECURITY_MINIMAL_DISPATCH,
    _SINGLETON_DISPATCH,
    _SYS_MINIMAL_DISPATCH,
    _parse_apm_policy_agent,
    _parse_data_group,
    _parse_gtm_pool,
    _parse_gtm_topology,
    _parse_gtm_wideip,
    _parse_header,
    _parse_ltm_auth,
    _parse_ltm_dns_cache_record,
    _parse_ltm_message_routing,
    _parse_ltm_minimal,
    _parse_minimal,
    _parse_monitor,
    _parse_node,
    _parse_pem_profile,
    _parse_persistence,
    _parse_policy,
    _parse_pool,
    _parse_profile,
    _parse_rule,
    _parse_security_minimal,
    _parse_snatpool,
    _parse_virtual,
    _parse_virtual_address,
)

# Public API


def parse_bigip_conf(source: str) -> BigipConfig:
    """Parse a BIG-IP configuration file and return a :class:`BigipConfig`.

    Handles ``ltm`` and ``gtm`` stanzas.  Unknown stanza types are silently
    skipped.
    """
    config = BigipConfig()
    blocks = _extract_blocks(source)
    source_map = DocumentBuffer.from_source(source)

    for block in blocks:
        # ``gtm topology`` carries a multi-token condition rather than a
        # full-path (``gtm topology ldns: subnet 10.0.0.0/8 server:
        # subnet 10.1.0.0/16 { ... }``); the standard header parser
        # would mis-tokenise it.  Pre-extract the condition and treat
        # the whole thing as the topology identifier.
        if block.header.startswith("gtm topology "):
            topo_id = block.header[len("gtm topology ") :].strip()
            config.gtm_topologies[topo_id] = _parse_gtm_topology(
                topo_id, block.body, source_map, block
            )
            continue
        generic = _parse_generic_header(block.header)
        if generic is not None:
            module_g, obj_type_g, identifier_g = generic
            generic_key = f"{module_g}::{obj_type_g}::{identifier_g or '<singleton>'}"
            config.generic_objects[generic_key] = BigipGenericObject(
                module=module_g,
                object_type=obj_type_g,
                identifier=identifier_g,
                header=block.header,
                range=_range_from_offsets(source_map, block.start_offset, block.end_offset),
            )

        parsed = _parse_header(block.header)
        if parsed is None:
            # Two-token / three-token bare singleton headers (e.g.
            # ``sys dns {``, ``auth password {``, ``net lacp-globals
            # {``) that the strict header parser rejects.  Dispatch
            # off ``generic`` via the unified singleton-typed and
            # minimal tables.
            if generic is not None:
                module_g, obj_type_g, identifier_g = generic
                if identifier_g != "":
                    continue
                singleton = _SINGLETON_DISPATCH.get((module_g, obj_type_g))
                if singleton is not None:
                    attr, parser_fn = singleton
                    getattr(config, attr)[""] = parser_fn(block.body, source_map, block)
                    continue
                # Bare-singleton minimal kinds (``sys httpd``, ``net
                # lacp-globals``, ``cm config-sync``, …) — route via
                # the per-module minimal dispatch.
                minimal_table = _MINIMAL_DISPATCH_BY_MODULE.get(module_g)
                if minimal_table is not None and obj_type_g in minimal_table[0]:
                    attr = minimal_table[0][obj_type_g]
                    parser_fn = minimal_table[1]
                    getattr(config, attr)[""] = parser_fn(
                        "",
                        block.body,
                        f"{module_g} {obj_type_g}",
                        source_map,
                        block,
                    )
                    continue
                # ``ltm`` minimal kinds aren't keyed in
                # ``_MINIMAL_DISPATCH_BY_MODULE``; their dispatch
                # table is :data:`_LTM_MINIMAL_DISPATCH`.
                if module_g == "ltm" and obj_type_g in _LTM_MINIMAL_DISPATCH:
                    attr = _LTM_MINIMAL_DISPATCH[obj_type_g]
                    getattr(config, attr)[""] = _parse_ltm_minimal(
                        "",
                        block.body,
                        f"ltm {obj_type_g}",
                        source_map,
                        block,
                    )
            continue
        module, obj_type, full_path = parsed

        # Generic minimal-dispatch pre-pass.  Runs first so that the
        # bundles 17-45 minimal kinds for non-ltm modules (net.* /
        # sys.* / apm.* / pem.* / cm.* / vcmp / cli / api-protection)
        # are dispatched without each module needing its own elif
        # chain.  Falls through to the module-specific blocks when
        # the kind isn't in any minimal table.
        _minimal_table = _MINIMAL_DISPATCH_BY_MODULE.get(module)
        if _minimal_table is not None and obj_type in _minimal_table[0]:
            _attr = _minimal_table[0][obj_type]
            _parser_fn = _minimal_table[1]
            getattr(config, _attr)[full_path] = _parser_fn(
                full_path,
                block.body,
                f"{module} {obj_type}",
                source_map,
                block,
            )
            continue

        # Named-typed dispatch — every (module, obj_type) pair listed
        # in :data:`_NAMED_DISPATCH` routes here with the canonical
        # ``(full_path, body, source_map, block)`` parser signature.
        # Singleton typed kinds (no full_path) are in
        # :data:`_SINGLETON_DISPATCH` but parsed via the same
        # branch when full_path is empty.
        named = _NAMED_DISPATCH.get((module, obj_type))
        if named is not None:
            attr, parser_fn = named
            getattr(config, attr)[full_path] = parser_fn(full_path, block.body, source_map, block)
            continue
        singleton = _SINGLETON_DISPATCH.get((module, obj_type))
        if singleton is not None and full_path == "":
            attr, singleton_parser = singleton
            getattr(config, attr)[""] = singleton_parser(block.body, source_map, block)
            continue

        # Family parsers that take an extra TMSH sub-type argument —
        # the parser signature deviates from the canonical 4-arg form
        # so they're handled inline rather than via dispatch tables.
        if module == "security" and obj_type in _SECURITY_MINIMAL_DISPATCH:
            attr = _SECURITY_MINIMAL_DISPATCH[obj_type]
            getattr(config, attr)[full_path] = _parse_security_minimal(
                full_path,
                block.body,
                f"security {obj_type}",
                source_map,
                block,
            )
            continue
        if module == "apm" and obj_type.startswith("policy agent "):
            agent_type = obj_type.rsplit(" ", 1)[-1]
            config.apm_policy_agents[full_path] = _parse_apm_policy_agent(
                full_path, block.body, source_map, block, agent_type
            )
            continue
        if module == "gtm" and obj_type.startswith("pool "):
            record_type = obj_type.split(" ", 1)[1]
            config.gtm_pools[full_path] = _parse_gtm_pool(
                full_path, block.body, record_type, source_map, block
            )
            continue
        if module == "gtm" and obj_type.startswith("wideip "):
            record_type = obj_type.split(" ", 1)[1]
            config.gtm_wideips[full_path] = _parse_gtm_wideip(
                full_path, block.body, record_type, source_map, block
            )
            continue
        if module == "pem" and obj_type in (
            "profile diameter-endpoint",
            "profile radius-aaa",
            "profile spm",
            "profile subscriber-mgmt",
        ):
            profile_type = obj_type.split(" ", 1)[1]
            config.pem_profiles[full_path] = _parse_pem_profile(
                full_path, profile_type, block.body, source_map, block
            )
            continue

        # Non-shared modules end here — only ``ltm`` and ``gtm`` fall
        # through to the legacy ``match`` block below (some kinds
        # like ``rule`` exist under both modules and dispatch by
        # ``module`` inside the match arms).
        if module not in ("ltm", "gtm"):
            continue

        # ltm/gtm shared kinds — ``data-group``, ``virtual``, ``pool``,
        # ``rule``, ``policy`` etc. that can appear under both modules.
        # The dispatch arms gate on ``module`` where the kind is
        # ltm-only.  Family parsers (profile / persistence / monitor
        # / dns-cache-records) stay below because they need an extra
        # sub-type argument.
        match obj_type:
            case "data-group internal":
                dg = _parse_data_group(
                    full_path, block.body, DataGroupType.INTERNAL, source_map, block
                )
                config.data_groups[full_path] = dg
            case "data-group external":
                dg = _parse_data_group(
                    full_path, block.body, DataGroupType.EXTERNAL, source_map, block
                )
                config.data_groups[full_path] = dg
            case "pool":
                if module == "ltm":
                    pool = _parse_pool(module, full_path, block.body, source_map, block)
                    config.pools[full_path] = pool
            case "virtual":
                vs = _parse_virtual(full_path, block.body, source_map, block)
                config.virtual_servers[full_path] = vs
            case "virtual-address":
                if module == "ltm":
                    va = _parse_virtual_address(full_path, block.body, source_map, block)
                    config.virtual_addresses[full_path] = va
            case _ if obj_type.startswith("dns cache records ") and module == "ltm":
                # Four-word kind: all five record sub-kinds (all / key /
                # msg / nameserver / rrset) merge into one container.
                record_kind = obj_type.rsplit(" ", 1)[1]
                config.ltm_dns_cache_records[full_path] = _parse_ltm_dns_cache_record(
                    full_path, block.body, record_kind, source_map, block
                )
            case _ if (
                module == "ltm" and obj_type.startswith("auth ") and obj_type in _LTM_AUTH_DISPATCH
            ):
                # Bundle 16 — ltm auth.*  All 11 kinds share one
                # parser; the ``kind`` field carries the full
                # ``ltm auth X`` label.
                attr = _LTM_AUTH_DISPATCH[obj_type]
                getattr(config, attr)[full_path] = _parse_ltm_auth(
                    full_path,
                    block.body,
                    f"ltm {obj_type}",
                    source_map,
                    block,
                )
            case _ if module == "ltm" and obj_type in _LTM_MINIMAL_DISPATCH:
                # Bundles 17-20 — generic ltm.* minimal-shape kinds
                # (CGNAT/LSN, global-settings singletons,
                # classification, tacdb).  Routed via the dispatch
                # table; ``kind`` field carries the TMSH label.
                attr = _LTM_MINIMAL_DISPATCH[obj_type]
                getattr(config, attr)[full_path] = _parse_ltm_minimal(
                    full_path,
                    block.body,
                    f"ltm {obj_type}",
                    source_map,
                    block,
                )
            case _ if (
                module == "ltm"
                and obj_type.startswith("message-routing ")
                and obj_type in _LTM_MESSAGE_ROUTING_DISPATCH
            ):
                # Bundle 15 — ltm message-routing.*  All 20 kinds
                # share one parser; the obj_type label picks the
                # right ``BigipConfig`` attribute via the dispatch
                # table.  The ``kind`` field on each instance carries
                # the full ``ltm message-routing X Y`` label.
                attr = _LTM_MESSAGE_ROUTING_DISPATCH[obj_type]
                getattr(config, attr)[full_path] = _parse_ltm_message_routing(
                    full_path,
                    block.body,
                    f"ltm {obj_type}",
                    source_map,
                    block,
                )
            case "node":
                node = _parse_node(full_path, block.body, source_map, block)
                config.nodes[full_path] = node
            case "snatpool":
                snatpool = _parse_snatpool(full_path, block.body, source_map, block)
                config.snat_pools[full_path] = snatpool
            case "rule":
                rule = _parse_rule(full_path, block.body, source_map, block)
                config.rules[full_path] = rule
            case "policy":
                if module == "ltm":
                    policy = _parse_policy(full_path, block.body, source_map, block)
                    config.policies[full_path] = policy
            case _ if obj_type.startswith("profile "):
                profile_type_str = obj_type.split(" ", 1)[1]
                profile = _parse_profile(full_path, profile_type_str, block.body, source_map, block)
                config.profiles[full_path] = profile
            case _ if obj_type.startswith("persistence "):
                persistence_type = obj_type.split(" ", 1)[1]
                persist = _parse_persistence(
                    full_path, persistence_type, block.body, source_map, block
                )
                config.persistence[full_path] = persist
            case _ if obj_type.startswith("monitor "):
                monitor_type = obj_type.split(" ", 1)[1]
                monitor = _parse_monitor(full_path, monitor_type, block.body, source_map, block)
                # Route by module — ``gtm monitor <type>`` lands in
                # ``gtm_monitors`` so it doesn't collide with the
                # identically-named LTM monitor (same path is valid
                # under both modules; TMSH enforces uniqueness per
                # (module, kind, full-path) tuple).
                if module == "gtm":
                    config.gtm_monitors[full_path] = monitor
                else:
                    config.monitors[full_path] = monitor

    return config


# Public API surface.  Everything else (parser helpers, dispatch
# tables, sub-parsers) is implementation detail and may change
# without notice.  Tests that need internals reach for them by
# their underscore-prefixed names.
