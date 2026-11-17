// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The range a live splice's body runs in, and the call it falls back to.

use super::CodegenCtx;
use tcl_bytecode::{Op, Operand, SpliceRange};
use tcl_compiler::ir::SpliceSite;
use tcl_lexer::Span;

/// The splices a function's statements stand in, and the ranges emitted.
#[derive(Debug, Default)]
pub struct SpliceEmission {
    sites: Vec<SpliceSite>,
    ranges: Vec<SpliceRange>,
    /// The site whose body is being emitted, and its start label.
    open: Option<(usize, String)>,
}

impl SpliceEmission {
    #[must_use]
    pub fn new(sites: Vec<SpliceSite>) -> Self {
        Self {
            sites,
            ..Self::default()
        }
    }

    /// The ranges emitted.
    #[must_use]
    pub fn into_ranges(self) -> Vec<SpliceRange> {
        self.ranges
    }
}

const fn within(outer: Span, inner: Span) -> bool {
    outer.start() <= inner.start() && inner.end() <= outer.end()
}

impl CodegenCtx<'_> {
    /// Before the statement at `span`: close the range of a body it is not
    /// part of, and open the range of a body it starts.
    pub(crate) fn splice_boundary(&mut self, span: Span) {
        let inside = self
            .splices
            .sites
            .iter()
            .position(|site| within(site.body_region, span));
        if self
            .splices
            .open
            .as_ref()
            .is_some_and(|(open, _)| inside != Some(*open))
        {
            self.close_splice();
        }
        if self.splices.open.is_none()
            && let Some(index) = inside
        {
            let start = self.fresh_label("splice_start");
            self.place_label(&start);
            self.splices.open = Some((index, start));
        }
    }

    /// After the last statement of a spliced body: jump over the call the
    /// splice falls back to, then emit that call from the words the splice
    /// evaluated, as the call itself would have been emitted, and record the
    /// range. A word with no slot records no range, and the function is then
    /// not admitted with the splice.
    pub(crate) fn close_splice(&mut self) {
        let Some((index, start)) = self.splices.open.take() else {
            return;
        };
        let site = self.splices.sites[index].clone();
        let slots: Option<Vec<u32>> = site
            .words
            .iter()
            .map(|word| {
                self.lvt
                    .find(word)
                    .and_then(|slot| u32::try_from(slot).ok())
            })
            .collect();
        // The body's value is the call's where the call's is read: its last
        // command's pop, past any empty command start, keeps it instead.
        if site.resumes_with_value
            && let Some(last) = self
                .instructions
                .iter_mut()
                .rev()
                .find(|instruction| instruction.op != Op::START_CMD)
            && last.op == Op::POP
        {
            last.op = Op::NOP;
        }
        let join = self.fresh_label("splice_join");
        self.emit(Op::JUMP4, vec![Operand::Label(join.clone())]);
        let end = self.fresh_label("splice_end");
        self.place_label(&end);
        self.set_command_source_span(site.call_span);
        for word in &site.words {
            self.load_var(word);
        }
        let count = super::bytecode_imm(site.words.len());
        let op = if count < 256 {
            Op::INVOKE_STK1
        } else {
            Op::INVOKE_STK4
        };
        self.emit(op, vec![Operand::Imm(count)]);
        if !site.resumes_with_value {
            self.emit(Op::POP, vec![]);
        }
        self.place_label(&join);
        if let Some(words) = slots {
            self.splices.ranges.push(SpliceRange {
                start,
                end,
                entry_depth: 0,
                words,
                resumes_with_value: site.resumes_with_value,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::ir::Statement;
    use tcl_registry::CommandRegistry;

    fn assign(name: &str, value: &str, span: Span) -> Statement {
        Statement::AssignConst {
            span,
            name: name.into(),
            name_braced: false,
            value: value.into(),
            value_span: None,
        }
    }

    /// The body runs between `start` and `end`; past it the code jumps over
    /// the call made from the slot holding the command word, which is where
    /// the runtime resumes when the body leaves abnormally. The call's value
    /// is the body's where it is read, and popped where it is not.
    #[test]
    fn a_spliced_body_is_followed_by_the_call_it_falls_back_to() {
        let registry = CommandRegistry::build_default();
        for resumes_with_value in [true, false] {
            let mut ctx = CodegenCtx::new(true, &[], &registry);
            ctx.lvt
                .hide_names_with_prefixes(vec![tcl_runtime_api::NameBytes::from(
                    "__inline_1__".as_bytes(),
                )]);
            let call = Span::new(0, 6);
            let body = Span::new(20, 30);
            ctx.splices = SpliceEmission::new(vec![SpliceSite {
                call_span: call,
                body_region: body,
                words: vec!["__inline_1__w0".into()],
                resumes_with_value,
            }]);
            for statement in [
                assign("__inline_1__w0", "vthree", call),
                assign("__inline_1__y", "3", Span::new(20, 30)),
            ] {
                ctx.splice_boundary(statement.span());
                ctx.emit_stmt_with_start_cmd(&statement, None, None);
            }
            ctx.close_splice();
            let ops: Vec<Op> = ctx.instructions.iter().map(|i| i.op).collect();
            let mut expected = vec![
                Op::PUSH1,
                Op::STORE_SCALAR1,
                Op::POP,
                Op::START_CMD,
                Op::PUSH1,
                Op::STORE_SCALAR1,
                if resumes_with_value { Op::NOP } else { Op::POP },
                Op::JUMP4,
                Op::LOAD_SCALAR1,
                Op::INVOKE_STK1,
            ];
            if !resumes_with_value {
                expected.push(Op::POP);
            }
            assert_eq!(ops, expected, "resumes_with_value {resumes_with_value}");
            let ranges = std::mem::take(&mut ctx.splices).into_ranges();
            let [range] = ranges.as_slice() else {
                panic!("one range: {ranges:?}");
            };
            assert_eq!(ctx.label_positions[&range.start], 3);
            assert_eq!(ctx.label_positions[&range.end], 8);
            assert_eq!(range.words, vec![0]);
            assert_eq!(range.entry_depth, 0);
            assert_eq!(range.resumes_with_value, resumes_with_value);
            assert_eq!(ctx.instructions[9].operands, vec![Operand::Imm(1)]);
            assert_eq!(ctx.instructions[9].source_span, Some(call));
        }
    }
}
