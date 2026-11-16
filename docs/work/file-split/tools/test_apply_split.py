from pathlib import Path

import pytest

from apply_split import PlanError, plan_file

SOURCE = """\
// Licence line.

//! Parent docs.

use std::fmt;

pub struct Walker {
    count: usize,
}

impl Walker {
    pub fn new() -> Self {
        Self { count: 0 }
    }

    // Counts one step.
    fn step(&mut self) {
        self.count += helper(self.count);
    }

    fn unused_elsewhere(&self) -> usize {
        super::other::scale(self.count)
    }
}

/// Adds one.
fn helper(value: usize) -> usize {
    value + 1
}

pub(crate) fn exported() -> usize {
    only_here()
}

fn only_here() -> usize {
    3
}

impl fmt::Display for Walker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.count)
    }
}

#[cfg(test)]
mod tests;
"""


def write(tmp_path: Path) -> Path:
    source = tmp_path / "src" / "walk.rs"
    source.parent.mkdir()
    source.write_text(SOURCE)
    (tmp_path / "src" / "walk").mkdir()
    (tmp_path / "src" / "walk" / "tests.rs").write_text("use super::*;\n#[test]\nfn t() { assert_eq!(only_here(), 3); }\n")
    return source


def plan(*modules: dict) -> dict:
    return {"file": "src/walk.rs", "module": list(modules)}


def test_moves_items_and_methods(tmp_path: Path) -> None:
    source = write(tmp_path)
    sizes = plan_file(
        tmp_path,
        plan(
            {"name": "stepping", "doc": "Stepping.", "items": ["impl:Walker::step..impl:Walker::unused_elsewhere", "fn:helper"]},
            {"name": "export", "items": ["fn:exported", "fn:only_here", "impl:fmt::Display for Walker"]},
        ),
        write=True,
    )
    assert [path for path, _ in sizes] == ["src/walk.rs", "src/walk/stepping.rs", "src/walk/export.rs"]
    parent = source.read_text()
    assert "use std::fmt;\n\nmod stepping;\nmod export;\n\n" in parent
    assert "pub(crate) use export::exported;" in parent
    assert "#[cfg(test)]\nuse export::only_here;" in parent
    assert "helper" not in parent
    assert "pub fn new()" in parent and "fn step" not in parent

    stepping = (tmp_path / "src/walk/stepping.rs").read_text()
    assert stepping.startswith("// Licence line.\n\n//! Stepping.\n\nuse super::*;\n\nimpl Walker {\n    // Counts one step.\n    fn step")
    assert "super::super::other::scale" in stepping
    assert "\n/// Adds one.\nfn helper" in stepping

    export = (tmp_path / "src/walk/export.rs").read_text()
    assert "pub(crate) fn exported" in export
    assert "pub(super) fn only_here" in export
    assert "impl fmt::Display for Walker {" in export


def test_a_method_named_outside_becomes_visible_to_the_parent(tmp_path: Path) -> None:
    write(tmp_path)
    plan_file(tmp_path, plan({"name": "stepping", "items": ["fn:helper"]}), write=True)
    assert "pub(super) fn helper" in (tmp_path / "src/walk/stepping.rs").read_text()
    assert "use stepping::helper;" in (tmp_path / "src/walk.rs").read_text()


def test_an_emptied_impl_block_is_removed(tmp_path: Path) -> None:
    source = write(tmp_path)
    plan_file(tmp_path, plan({"name": "all", "items": ["impl:Walker::new..impl:Walker::unused_elsewhere"]}), write=True)
    assert "impl Walker" not in source.read_text()


def test_check_writes_nothing(tmp_path: Path) -> None:
    source = write(tmp_path)
    plan_file(tmp_path, plan({"name": "stepping", "items": ["fn:helper"]}), write=False)
    assert source.read_text() == SOURCE
    assert not (tmp_path / "src/walk/stepping.rs").exists()


@pytest.mark.parametrize(
    ("items", "message"),
    [
        (["fn:missing"], "no item"),
        (["use:std::fmt"], "cannot move"),
        (["impl:fmt::Display for Walker::fmt"], "trait impl"),
        (["fn:helper", "fn:helper"], "in both"),
        (["impl:Walker", "impl:Walker::step"], "whole impl"),
    ],
)
def test_refuses_a_bad_plan(tmp_path: Path, items: list[str], message: str) -> None:
    write(tmp_path)
    with pytest.raises(PlanError, match=message):
        plan_file(tmp_path, plan({"name": "stepping", "items": items}), write=True)
