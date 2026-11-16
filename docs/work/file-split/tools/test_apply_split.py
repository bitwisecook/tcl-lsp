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


def test_fields_and_methods_used_outside_a_moved_type_are_widened(tmp_path: Path) -> None:
    source = tmp_path / "src" / "ctx.rs"
    source.parent.mkdir()
    source.write_text(
        """\
struct Ctx {
    depth: usize,
    /// Only the type's own impl reads this.
    hidden: usize,
    pub(super) shared: usize,
}

impl Ctx {
    fn deeper(&self) -> usize {
        self.hidden + 1
    }

    fn internal(&self) -> usize {
        self.hidden
    }
}

fn walk(ctx: &Ctx) -> usize {
    ctx.depth + ctx.deeper()
}
"""
    )
    plan_file(tmp_path, {"file": "src/ctx.rs", "module": [{"name": "context", "items": ["struct:Ctx", "impl:Ctx"]}]}, write=True)
    context = (tmp_path / "src/ctx/context.rs").read_text()
    assert "    pub(super) depth: usize," in context
    assert "\n    hidden: usize," in context
    assert "    pub(in super::super) shared: usize," in context
    assert "    pub(super) fn deeper(" in context
    assert "\n    fn internal(" in context
    assert "use context::Ctx;" in source.read_text()


def test_a_private_type_in_an_exposed_signature_is_widened_and_imports_are_grouped(tmp_path: Path) -> None:
    source = tmp_path / "src" / "roles.rs"
    source.parent.mkdir()
    source.write_text(
        """\
struct Ambiguity(usize);

struct Unrelated(usize);

fn ambiguity() -> Ambiguity {
    let _ = Unrelated(1);
    Ambiguity(0)
}

fn other() -> usize {
    1
}

fn caller() -> usize {
    ambiguity().0 + other()
}
"""
    )
    plan_file(
        tmp_path,
        {"file": "src/roles.rs", "module": [{"name": "ambiguous", "items": ["struct:Ambiguity..fn:other"]}]},
        write=True,
    )
    child = (tmp_path / "src/roles/ambiguous.rs").read_text()
    assert "pub(super) struct Ambiguity" in child
    assert "\nstruct Unrelated" in child
    assert "use ambiguous::{ambiguity, other};" in source.read_text()


def test_a_module_of_tests_is_declared_for_tests_only(tmp_path: Path) -> None:
    source = tmp_path / "src" / "names.rs"
    source.parent.mkdir()
    body = "fn name() -> usize {\n    1\n}\n\n#[test]\nfn names_one() {\n    assert_eq!(name(), 1);\n}\n"
    source.write_text(body)
    plan_file(tmp_path, {"file": "src/names.rs", "module": [{"name": "name_tests", "items": ["fn:names_one"]}]}, write=True)
    assert "#[cfg(test)]\nmod name_tests;" in source.read_text()

    (tmp_path / "src" / "lib.rs").write_text("#[cfg(test)]\nmod cases;\n")
    cases = tmp_path / "src" / "cases.rs"
    cases.write_text(body)
    plan_file(tmp_path, {"file": "src/cases.rs", "module": [{"name": "naming", "items": ["fn:names_one"]}]}, write=True)
    assert "\nmod naming;" in cases.read_text() and "cfg(test)" not in cases.read_text()


def test_inline_modules_move_and_hoist(tmp_path: Path) -> None:
    source = tmp_path / "tests" / "it" / "cases.rs"
    source.parent.mkdir(parents=True)
    source.write_text(
        """\
fn shared() -> usize {
    1
}

mod joins {
    use super::*;

    #[test]
    fn joins_one() {
        assert_eq!(shared(), 1);
    }
}

mod sinks {
    use super::*;

    #[test]
    fn sinks_one() {
        assert_eq!(shared(), 1);
    }
}

mod big {
    use super::*;

    const TEXT: &str = include_str!("data.txt");
}
"""
    )
    plan_file(
        tmp_path,
        {"file": "tests/it/cases.rs", "hoist": ["big"], "module": [{"name": "flow", "items": ["mod:joins..mod:sinks"]}]},
        write=True,
    )
    parent = source.read_text()
    assert "mod big;" in parent and "mod flow;" in parent and "mod joins" not in parent
    flow = (tmp_path / "tests/it/cases/flow.rs").read_text()
    assert "\nmod joins {\n    use super::super::*;" in flow and "\nmod sinks {" in flow
    big = (tmp_path / "tests/it/cases/big.rs").read_text()
    assert big.startswith("    use super::*;") and 'include_str!("../data.txt")' in big


def test_a_section_heading_moves_with_the_first_item_of_its_section(tmp_path: Path) -> None:
    source = tmp_path / "src" / "sections.rs"
    source.parent.mkdir()
    source.write_text(
        """\
// Licence line.

fn stays() {}

// ---- Loops ----
// Everything about loops.

/// Runs a loop.
fn run_loop() {}

fn loop_step() {}

// ---- Calls ----

fn call() {}
"""
    )
    plan_file(tmp_path, {"file": "src/sections.rs", "module": [{"name": "loops", "items": ["fn:run_loop", "fn:loop_step"]}]}, write=True)
    loops = (tmp_path / "src/sections/loops.rs").read_text()
    assert "// ---- Loops ----\n// Everything about loops.\n\n/// Runs a loop.\nfn run_loop" in loops
    parent = source.read_text()
    assert "Loops" not in parent and "// ---- Calls ----\n\nfn call() {}" in parent
    assert parent.startswith("// Licence line.\n")


def test_declarations_follow_the_inner_docs_and_includes_follow_the_move(tmp_path: Path) -> None:
    source = tmp_path / "src" / "ops.rs"
    source.parent.mkdir()
    source.write_text(
        """\
// Licence line.

//! Operations.
//! More.

type Alias = usize;

const WIDTH: usize = if cfg!(test) { 1 } else { 2 };

fn text() -> &'static str {
    include_str!("data/ops.txt")
}
"""
    )
    plan_file(tmp_path, {"file": "src/ops.rs", "module": [{"name": "texts", "items": ["const:WIDTH", "fn:text"]}]}, write=True)
    parent = source.read_text()
    assert parent.startswith("// Licence line.\n\n//! Operations.\n//! More.\n\nmod texts;\n")
    texts = (tmp_path / "src/ops/texts.rs").read_text()
    assert "const WIDTH: usize = if cfg!(test) { 1 } else { 2 };" in texts
    assert 'include_str!("../data/ops.txt")' in texts


def test_an_import_carries_the_conditions_of_its_item(tmp_path: Path) -> None:
    source = tmp_path / "src" / "trace.rs"
    source.parent.mkdir()
    source.write_text(
        """\
#[cfg(debug_assertions)]
pub(crate) fn trace() {}

#[cfg(test)]
pub(crate) fn fixture() {}

fn run() {
    #[cfg(debug_assertions)]
    trace();
}
"""
    )
    plan_file(tmp_path, {"file": "src/trace.rs", "module": [{"name": "tracing", "items": ["fn:trace", "fn:fixture"]}]}, write=True)
    parent = source.read_text()
    assert "#[cfg(debug_assertions)]\npub(crate) use tracing::trace;" in parent
    assert "#[cfg(test)]\npub(crate) use tracing::fixture;" in parent


def test_an_inline_module_keeps_naming_what_moved_with_it(tmp_path: Path) -> None:
    source = tmp_path / "src" / "scan.rs"
    source.parent.mkdir()
    source.write_text(
        """\
struct Window(usize);

fn other() {}

#[cfg(test)]
mod window_controls {
    use super::Window;
    use super::other;
}
"""
    )
    plan_file(tmp_path, {"file": "src/scan.rs", "module": [{"name": "batches", "items": ["struct:Window", "mod:window_controls"]}]}, write=True)
    batches = (tmp_path / "src/scan/batches.rs").read_text()
    assert "    use super::Window;\n    use super::super::other;" in batches
