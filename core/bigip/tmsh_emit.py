"""Emit a sequence of ``tmsh create / modify`` commands from a parsed config.

Powers ``f5 tmsh``.  The output is a tmsh script that can be pasted
into a BIG-IP shell to recreate every object in the input — useful for
migrating a config from one device to another, replaying just the
matched subset of an ``f5 grep`` result, or building a clean fixture.

Order matters: tmsh refuses to create an object that references a
target which doesn't yet exist, so we emit objects in dependency
order:

  1. partitions, route-domains, vlans, self IPs
  2. nodes
  3. monitors
  4. data-groups
  5. profiles
  6. snat-pools, persistence
  7. pools
  8. iRules
  9. virtuals

Each object renders as either a single ``tmsh create`` command (with
all its properties on one line in tmsh syntax) or, for iRules with
embedded newlines, a ``tmsh create ... rules-rule { ... }`` block.

The emitter is best-effort: properties not modelled by
:class:`BigipConfig` are pulled directly from the original SCF stanza
text (via the same block extractor :mod:`core.bigip.emit` uses) so
nothing is silently lost.  When a stanza can't be reconstructed, the
emitter falls back to emitting it as ``tmsh load sys config from-terminal
... <<EOT ... EOT`` so the user still gets a working command.
"""

from __future__ import annotations

import re
from dataclasses import dataclass

from .emit import iter_blocks_with_source
from .model import (
    BigipConfig,
    BigipDataGroup,
    BigipMonitor,
    BigipNode,
    BigipPersistence,
    BigipPool,
    BigipProfile,
    BigipRule,
    BigipSnatPool,
    BigipVirtualServer,
    DataGroupType,
)

# Order in which kinds get emitted.  Generic objects (route-domains,
# vlans, partitions, ...) come first so any LTM object that references
# them resolves cleanly.
_DEFAULT_ORDER = (
    "generic-foundation",
    "node",
    "monitor",
    "data-group",
    "profile",
    "snatpool",
    "persistence",
    "pool",
    "rule",
    "virtual",
)

# (module, object_type) pairs that must be created before any LTM object.
_FOUNDATION_TYPES: frozenset[tuple[str, str]] = frozenset(
    {
        ("auth", "partition"),
        ("net", "route-domain"),
        ("net", "vlan"),
        ("net", "self"),
        ("net", "trunk"),
        ("net", "interface"),
    }
)


@dataclass(frozen=True, slots=True)
class TmshScript:
    text: str
    command_count: int


def emit_tmsh(
    cfg: BigipConfig,
    *,
    source: str = "",
    include: tuple[str, ...] = _DEFAULT_ORDER,
    use_modify_for_existing: bool = False,
) -> TmshScript:
    """Render *cfg* as a tmsh script.

    *source* is the original SCF text, used to recover stanza body for
    objects that the model doesn't fully capture (so e.g. an iRule's
    full Tcl source survives the round-trip).

    *include* lets callers narrow the kinds emitted (handy when
    feeding the output of ``f5 grep`` back through ``f5 tmsh``).

    *use_modify_for_existing* swaps ``create`` for ``modify`` so the
    script edits already-present objects in place — useful for partial
    config sync.
    """
    lines: list[str] = []
    count = 0
    verb = "modify" if use_modify_for_existing else "create"

    block_text_by_id: dict[tuple[str, str, str], str] = {}
    for block_slice in iter_blocks_with_source(source):
        block_text_by_id[(block_slice.module, block_slice.object_type, block_slice.identifier)] = (
            block_slice.text
        )

    def _add(line: str) -> None:
        nonlocal count
        lines.append(line)
        count += 1

    if "generic-foundation" in include:
        for ident, obj in cfg.generic_objects.items():
            if (obj.module, obj.object_type) not in _FOUNDATION_TYPES:
                continue
            _add(_render_generic(verb, obj, block_text_by_id))

    if "node" in include:
        for node in cfg.nodes.values():
            _add(_render_node(verb, node))

    if "monitor" in include:
        for monitor in cfg.monitors.values():
            _add(_render_monitor(verb, monitor, source, block_text_by_id))

    if "data-group" in include:
        for dg in cfg.data_groups.values():
            _add(_render_data_group(verb, dg))

    if "profile" in include:
        for profile in cfg.profiles.values():
            _add(_render_profile(verb, profile, block_text_by_id))

    if "snatpool" in include:
        for sp in cfg.snat_pools.values():
            _add(_render_snatpool(verb, sp))

    if "persistence" in include:
        for pers in cfg.persistence.values():
            _add(_render_persistence(verb, pers, block_text_by_id))

    if "pool" in include:
        for pool in cfg.pools.values():
            _add(_render_pool(verb, pool))

    if "rule" in include:
        for rule in cfg.rules.values():
            _add(_render_rule(verb, rule))

    if "virtual" in include:
        for vs in cfg.virtual_servers.values():
            _add(_render_virtual(verb, vs))

    text = "\n".join(lines) + ("\n" if lines else "")
    return TmshScript(text=text, command_count=count)


def emit_tmsh_delta(
    old_cfg: BigipConfig,
    new_cfg: BigipConfig,
    *,
    old_source: str = "",
    new_source: str = "",
    include: tuple[str, ...] = _DEFAULT_ORDER,
) -> TmshScript:
    """Emit only the *changed* objects between *old_cfg* and *new_cfg*.

    Produces a tighter tmsh script than :func:`emit_tmsh` for the
    ``f5 query`` change-rollout shape: rather than re-rendering every
    object in the config (which the full emitter does and which
    produces a ~126-line script even when only two objects actually
    changed in the test report's 1906-line corkscrew config), this
    walks each per-kind container twice and emits:

    - ``tmsh create <kind> <full-path> { ... }`` for objects added in
      *new_cfg* but absent from *old_cfg*;
    - ``tmsh modify <kind> <full-path> { ... }`` for objects whose
      stanza text differs between *old_source* and *new_source*;
    - ``tmsh delete <kind> <full-path>`` for objects removed in
      *new_cfg* that were present in *old_cfg*.

    Equality is compared on the parsed stanza text (via the block
    iterator the SCF emitter also uses): if the textual stanza is
    byte-identical the object is treated as unchanged and dropped.
    Falls back to dataclass equality when neither source is supplied.

    Order:

    1. **Modifies** for every kind (in dependency order: nodes →
       monitors → ... → virtuals).  Modifies typically rewire
       references away from objects that are about to be deleted,
       so they fire first.
    2. **Deletes** for every kind (in reverse dependency order:
       virtuals → rules → pools → ...).  Running deletes before
       creates is the rename-shaped diff fix: a renamed virtual
       (delete old / create new sharing the same destination)
       would collide on the device's unique-listener constraint if
       we created the new stanza while the old was still bound.
       Now the old listener releases first.
    3. **Creates** for every kind (in dependency order).  By this
       point any uniqueness collisions with old objects have been
       resolved and any references the new objects need are
       already in place (foundation creates come first).

    Foundation modifies / deletes are interleaved at the right ends
    of phases 1-3 so partition / vlan / route-domain creates
    precede any LTM creates that reference them.
    """
    new_blocks = _index_blocks(new_source)
    old_blocks = _index_blocks(old_source)

    def _changed(kind_tuple: tuple[str, str, str]) -> bool:
        return new_blocks.get(kind_tuple, "") != old_blocks.get(kind_tuple, "")

    lines: list[str] = []
    count = 0

    def _add(line: str) -> None:
        nonlocal count
        lines.append(line)
        count += 1

    kind_table = (
        ("node", "nodes", _render_node, False),
        ("monitor", "monitors", _render_monitor, True),
        ("data-group", "data_groups", _render_data_group, False),
        ("profile", "profiles", _render_profile, False),
        ("snatpool", "snat_pools", _render_snatpool, False),
        ("persistence", "persistence", _render_persistence, True),
        ("pool", "pools", _render_pool, False),
        ("rule", "rules", _render_rule, False),
        ("virtual", "virtual_servers", _render_virtual, False),
    )

    # ── 1. Modifies (foundation first, then dependency order) ─────
    if "generic-foundation" in include:
        for ident, obj in new_cfg.generic_objects.items():
            if (obj.module, obj.object_type) not in _FOUNDATION_TYPES:
                continue
            key = (obj.module, obj.object_type, obj.identifier)
            old = old_cfg.generic_objects.get(ident)
            if old is not None and _changed(key):
                _add(_render_generic("modify", obj, new_blocks))
    for kind_name, container_attr, render_fn, _src in kind_table:
        if kind_name not in include:
            continue
        new_container = getattr(new_cfg, container_attr)
        old_container = getattr(old_cfg, container_attr)
        for full_path, obj in new_container.items():
            old_obj = old_container.get(full_path)
            if old_obj is None:
                continue  # creates handled in phase 3
            key = _block_key_for(kind_name, obj)
            if _changed(key):
                _add(_dispatch_render("modify", render_fn, obj, new_source, new_blocks))

    # ── 2. Deletes (reverse dependency order) ─────────────────────
    delete_kinds = [k for k in reversed(include) if k not in ("generic-foundation",)]
    for kind_name in delete_kinds:
        container_attr = {
            "node": "nodes",
            "monitor": "monitors",
            "data-group": "data_groups",
            "profile": "profiles",
            "snatpool": "snat_pools",
            "persistence": "persistence",
            "pool": "pools",
            "rule": "rules",
            "virtual": "virtual_servers",
        }.get(kind_name)
        if container_attr is None:
            continue
        new_container = getattr(new_cfg, container_attr)
        old_container = getattr(old_cfg, container_attr)
        for full_path, old_obj in old_container.items():
            if full_path in new_container:
                continue
            _add(_delete_command(kind_name, old_obj))
    # Generic foundation deletes (deferred to end of phase 2 so
    # they don't strand references in objects deleted above).
    if "generic-foundation" in include:
        for ident, obj in old_cfg.generic_objects.items():
            if (obj.module, obj.object_type) not in _FOUNDATION_TYPES:
                continue
            if ident in new_cfg.generic_objects:
                continue
            _add(f"tmsh delete {obj.module} {obj.object_type} {obj.identifier}")

    # ── 3. Creates (foundation first, then dependency order) ─────
    # All uniqueness collisions with old objects are gone now —
    # phase 2 cleared the old stanzas, so the rename-shaped diff's
    # ``create new virtual same destination`` lands without a
    # listener collision.
    if "generic-foundation" in include:
        for ident, obj in new_cfg.generic_objects.items():
            if (obj.module, obj.object_type) not in _FOUNDATION_TYPES:
                continue
            if ident in old_cfg.generic_objects:
                continue
            _add(_render_generic("create", obj, new_blocks))
    for kind_name, container_attr, render_fn, _src in kind_table:
        if kind_name not in include:
            continue
        new_container = getattr(new_cfg, container_attr)
        old_container = getattr(old_cfg, container_attr)
        for full_path, obj in new_container.items():
            if full_path in old_container:
                continue  # modify, already emitted in phase 1
            _add(_dispatch_render("create", render_fn, obj, new_source, new_blocks))

    text = "\n".join(lines) + ("\n" if lines else "")
    return TmshScript(text=text, command_count=count)


def _index_blocks(source: str) -> dict[tuple[str, str, str], str]:
    if not source:
        return {}
    out: dict[tuple[str, str, str], str] = {}
    for bs in iter_blocks_with_source(source):
        out[(bs.module, bs.object_type, bs.identifier)] = bs.text
    return out


def _block_key_for(kind_name: str, obj: object) -> tuple[str, str, str]:
    """Return the (module, object_type, identifier) tuple matching block-index keys.

    Mirrors what :func:`iter_blocks_with_source` produces so the
    delta emitter can look up the stanza text for an object given
    its dataclass.  Each *kind_name* maps to a fixed (module,
    object_type) pair; *identifier* is the object's full-path.
    """
    type_map = {
        "node": ("ltm", "node"),
        "monitor": ("ltm", "monitor"),
        "data-group": ("ltm", "data-group"),
        "profile": ("ltm", "profile"),
        "snatpool": ("ltm", "snatpool"),
        "persistence": ("ltm", "persistence"),
        "pool": ("ltm", "pool"),
        "rule": ("ltm", "rule"),
        "virtual": ("ltm", "virtual"),
    }
    module, object_type = type_map[kind_name]
    full_path = getattr(obj, "full_path", "")
    return (module, object_type, full_path)


def _dispatch_render(verb: str, render_fn, obj, new_source: str, new_blocks: dict) -> str:
    """Call a per-kind renderer with whatever extra args it expects.

    Most renderers take ``(verb, obj)``; iRules / persistence /
    monitors that may carry block-text bodies take a *source* and
    *block_text_by_id* argument set too.  We dispatch by inspecting
    the function signature lazily to keep the call sites compact.
    """
    import inspect

    sig = inspect.signature(render_fn)
    params = sig.parameters
    if len(params) == 2:
        return render_fn(verb, obj)
    # Heuristic for the (verb, obj, source, blocks) and (verb, obj, blocks) shapes.
    if "source" in params:
        return render_fn(verb, obj, new_source, new_blocks)
    return render_fn(verb, obj, new_blocks)


def _delete_command(kind_name: str, obj: object) -> str:
    """Build a ``tmsh delete <kind> <full-path>`` command."""
    full_path = getattr(obj, "full_path", "")
    spelt = {
        "node": "ltm node",
        "monitor": "ltm monitor",
        "data-group": "ltm data-group",
        "profile": "ltm profile",
        "snatpool": "ltm snatpool",
        "persistence": "ltm persistence",
        "pool": "ltm pool",
        "rule": "ltm rule",
        "virtual": "ltm virtual",
    }.get(kind_name, f"ltm {kind_name}")
    return f"tmsh delete {spelt} {full_path}"


# ── Renderers ───────────────────────────────────────────────────────


def _quote(value: str) -> str:
    """Quote *value* for tmsh if it contains whitespace or braces."""
    if not value:
        return '""'
    if any(ch in value for ch in (" ", "\t", "{", "}", '"')):
        escaped = value.replace("\\", "\\\\").replace('"', '\\"')
        return f'"{escaped}"'
    return value


def _render_node(verb: str, node: BigipNode) -> str:
    fields = []
    if node.address:
        fields.append(f"address {node.address}")
    body = " ".join(fields)
    return f"tmsh {verb} ltm node {node.full_path}" + (f" {{ {body} }}" if body else "")


def _render_pool(verb: str, pool: BigipPool) -> str:
    fields: list[str] = []
    if pool.load_balancing_mode:
        fields.append(f"load-balancing-mode {pool.load_balancing_mode}")
    if pool.monitor:
        fields.append(f"monitor {_quote(pool.monitor)}")
    if pool.members:
        member_lines = []
        for m in pool.members:
            inner = []
            if m.address:
                inner.append(f"address {m.address}")
            if m.monitor:
                inner.append(f"monitor {_quote(m.monitor)}")
            inner_str = " ".join(inner)
            member_lines.append(f"{m.name} {{ {inner_str} }}" if inner_str else f"{m.name} {{ }}")
        fields.append("members { " + " ".join(member_lines) + " }")
    body = " ".join(fields)
    return f"tmsh {verb} ltm pool {pool.full_path}" + (f" {{ {body} }}" if body else "")


def _render_monitor(
    verb: str,
    monitor: BigipMonitor,
    source: str,
    block_text_by_id: dict[tuple[str, str, str], str],
) -> str:
    monitor_type = monitor.monitor_type or "tcp"
    # Recover any extra properties (send/recv strings, intervals) from the
    # original block text.  The model only captures the type.
    props = _extract_block_properties(
        block_text_by_id.get(("ltm", f"monitor {monitor_type}", monitor.full_path))
        or block_text_by_id.get(("ltm", "monitor", monitor.full_path))
        or ""
    )
    body = " ".join(f"{k} {_quote(v)}" for k, v in props.items())
    head = f"tmsh {verb} ltm monitor {monitor_type} {monitor.full_path}"
    return head + (f" {{ {body} }}" if body else "")


def _render_data_group(verb: str, dg: BigipDataGroup) -> str:
    kind = "internal" if dg.kind == DataGroupType.INTERNAL else "external"
    parts: list[str] = []
    if dg.value_type:
        parts.append(f"type {dg.value_type}")
    if dg.records:
        record_block = " ".join(f"{r} {{ }}" for r in dg.records)
        parts.append(f"records {{ {record_block} }}")
    body = " ".join(parts)
    head = f"tmsh {verb} ltm data-group {kind} {dg.full_path}"
    return head + (f" {{ {body} }}" if body else "")


def _render_profile(
    verb: str,
    profile: BigipProfile,
    block_text_by_id: dict[tuple[str, str, str], str],
) -> str:
    profile_type = profile.profile_type.name.lower().replace("_", "-")
    return (
        f"tmsh {verb} ltm profile {profile_type} {profile.full_path} "
        + _passthrough_body(block_text_by_id, "ltm", f"profile {profile_type}", profile.full_path)
    ).rstrip()


def _render_snatpool(verb: str, sp: BigipSnatPool) -> str:
    body = ""
    if sp.members:
        body = "members { " + " ".join(sp.members) + " }"
    return f"tmsh {verb} ltm snatpool {sp.full_path}" + (f" {{ {body} }}" if body else "")


def _render_persistence(
    verb: str,
    pers: BigipPersistence,
    block_text_by_id: dict[tuple[str, str, str], str],
) -> str:
    persistence_type = pers.persistence_type or "source-addr"
    return (
        f"tmsh {verb} ltm persistence {persistence_type} {pers.full_path} "
        + _passthrough_body(
            block_text_by_id, "ltm", f"persistence {persistence_type}", pers.full_path
        )
    ).rstrip()


def _render_rule(verb: str, rule: BigipRule) -> str:
    """Render an iRule as a tmsh create command.

    Tcl bodies routinely span many lines and embed braces; the cleanest
    way is to keep the original body verbatim inside a single
    ``rules-rule { ... }`` block.
    """
    body = rule.source.strip()
    return f"tmsh {verb} ltm rule {rule.full_path} {{\n{body}\n}}"


def _render_virtual(verb: str, vs: BigipVirtualServer) -> str:
    fields: list[str] = []
    if vs.destination:
        fields.append(f"destination {vs.destination}")
    if vs.pool:
        fields.append(f"pool {vs.pool}")
    if vs.profiles:
        fields.append("profiles { " + " ".join(f"{p} {{ }}" for p in vs.profiles) + " }")
    if vs.rules:
        fields.append("rules { " + " ".join(vs.rules) + " }")
    if vs.persist:
        fields.append("persist { " + " ".join(f"{p} {{ }}" for p in vs.persist) + " }")
    if vs.snatpool:
        fields.append(f"snatpool {vs.snatpool}")
    if vs.source_address_translation:
        fields.append(f"source-address-translation {{ type {vs.source_address_translation} }}")
    body = " ".join(fields)
    return f"tmsh {verb} ltm virtual {vs.full_path}" + (f" {{ {body} }}" if body else "")


def _render_generic(
    verb: str,
    obj,  # noqa: ANN001
    block_text_by_id: dict[tuple[str, str, str], str],
) -> str:
    """Fall back to the original block body for objects we don't fully model."""
    body_text = _passthrough_body(block_text_by_id, obj.module, obj.object_type, obj.identifier)
    head = f"tmsh {verb} {obj.module} {obj.object_type} {obj.identifier}".rstrip()
    return f"{head} {body_text}".rstrip() if body_text else head


def _passthrough_body(
    block_text_by_id: dict[tuple[str, str, str], str],
    module: str,
    object_type: str,
    identifier: str,
) -> str:
    """Return the original ``{ ... }`` body of a block, collapsed to a single line."""
    text = block_text_by_id.get((module, object_type, identifier))
    if not text:
        return ""
    match = re.search(r"\{(.*)\}\s*\Z", text, re.DOTALL)
    if not match:
        return ""
    inner = match.group(1).strip()
    if not inner:
        return ""
    # Collapse newlines to spaces (tmsh on the CLI accepts a single line).
    collapsed = re.sub(r"\s+", " ", inner)
    return f"{{ {collapsed} }}"


def _extract_block_properties(text: str) -> dict[str, str]:
    """Cheap top-level prop extractor for monitor stanzas (no nesting)."""
    if not text:
        return {}
    match = re.search(r"\{(.*)\}\s*\Z", text, re.DOTALL)
    if not match:
        return {}
    inner = match.group(1)
    out: dict[str, str] = {}
    for raw_line in inner.splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        if "{" in line or "}" in line:
            continue  # skip nested
        if " " not in line:
            out[line] = ""
            continue
        key, value = line.split(None, 1)
        out[key] = value.strip()
    return out
