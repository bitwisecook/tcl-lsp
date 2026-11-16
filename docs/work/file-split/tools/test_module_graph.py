"""Tests for module_graph.py."""

from pathlib import Path

from module_graph import components, dependencies, layers


def write(root: Path, files: dict[str, str]) -> None:
    for name, text in files.items():
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")


def test_a_leaf_a_cycle_and_what_sits_on_them(tmp_path: Path) -> None:
    write(
        tmp_path,
        {
            "lib.rs": "pub mod leaf;\npub mod a;\npub mod b;\npub mod top;\npub use a::Thing;\n",
            "leaf.rs": "pub fn one() -> u8 { 1 }\n",
            "a.rs": "use crate::b::other;\npub struct Thing;\n",
            "b/mod.rs": "use super::a::Thing;\npub fn other() {}\n",
            "b/inner.rs": "use crate::leaf::one;\n",
            "top.rs": "use crate::Thing;\n",
        },
    )
    deps, lines = dependencies(tmp_path)
    assert deps == {"leaf": set(), "a": {"b"}, "b": {"a", "leaf"}, "top": {"a"}}
    assert lines["leaf"] == 1
    found = components(deps)
    level = dict(zip((frozenset(component) for component in found), layers(deps, found)))
    assert level == {frozenset({"leaf"}): 0, frozenset({"a", "b"}): 1, frozenset({"top"}): 2}


def test_test_code_makes_no_dependency(tmp_path: Path) -> None:
    write(
        tmp_path,
        {
            "lib.rs": "pub mod a;\npub mod b;\n",
            "a.rs": "pub fn f() {}\n#[cfg(test)]\nmod tests {\n    use crate::b::g;\n}\n",
            "a/tests.rs": "use crate::b::g;\n",
            "b.rs": "pub fn g() {}\n",
        },
    )
    deps, _ = dependencies(tmp_path)
    assert deps == {"a": set(), "b": set()}
