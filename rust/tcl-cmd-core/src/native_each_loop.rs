// SPDX-License-Identifier: AGPL-3.0-or-later
//! Resumable generic each-loop scheduling over original backend objects.

use crate::CmdError;
use tcl_dialect::TclVersion;
use tcl_runtime_api::{
    Code,
    native_each_loop::{NativeEachLoopKind, NativeEachLoopRecipe},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preparation {
    Variables(usize),
    Values(usize),
}

/// Native preparation order. Body preparation is deliberately absent.
#[must_use]
pub fn preparation(recipe: NativeEachLoopRecipe, groups: usize) -> Vec<Preparation> {
    if recipe.variables_first() {
        (0..groups).map(Preparation::Variables).collect()
    } else {
        (0..groups)
            .flat_map(|group| [Preparation::Variables(group), Preparation::Values(group)])
            .collect()
    }
}

#[must_use]
pub fn empty_variables(recipe: NativeEachLoopRecipe, kind: NativeEachLoopKind) -> CmdError {
    let name = if recipe.live_iterators() {
        "foreach"
    } else {
        kind.name()
    };
    let message = format!("{name} varlist is empty");
    let error = match recipe {
        NativeEachLoopRecipe::C(TclVersion::V8_6 | TclVersion::V9_0 | TclVersion::V9_1) => {
            CmdError::with_error_code(
                message,
                format!("TCL OPERATION {} NEEDVARS", name.to_ascii_uppercase()),
            )
        }
        NativeEachLoopRecipe::C(TclVersion::V8_4 | TclVersion::V8_5)
        | NativeEachLoopRecipe::Jim084 => CmdError::new(message),
    };
    match recipe {
        NativeEachLoopRecipe::C(version) => error
            .with_native_string_result(tcl_syntax::native_string::NativeStringProtocol::C(version)),
        NativeEachLoopRecipe::Jim084 => error,
    }
}

/// Selected generic setter diagnostic; variable lookup owns the original error.
pub enum SetterFailure {
    Preserve,
    Replace(CmdError),
    Context(Vec<u8>),
}
#[must_use]
pub fn setter_failure(
    recipe: NativeEachLoopRecipe,
    kind: NativeEachLoopKind,
    original: &[u8],
) -> SetterFailure {
    let name = tcl_core_types::c_string_extent(original);
    match recipe {
        NativeEachLoopRecipe::Jim084 => SetterFailure::Preserve,
        NativeEachLoopRecipe::C(TclVersion::V8_4) => {
            let mut message = b"couldn't set loop variable: \"".to_vec();
            message.extend_from_slice(name);
            message.push(b'"');
            SetterFailure::Replace(CmdError::new_bytes(message))
        }
        NativeEachLoopRecipe::C(_) => {
            let mut context =
                format!("\n    (setting {} loop variable \"", kind.name()).into_bytes();
            context.extend_from_slice(name);
            context.extend_from_slice(b"\")");
            SetterFailure::Context(context)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EachLoopAction {
    /// Inspect the next original Jim value iterator for exhaustion, in order.
    Check(usize),
    /// Refresh the selected originals before this group's setters.
    Refresh(usize),
    /// Original variable/member indices. None selects native padding.
    Assign {
        group: usize,
        variable: usize,
        value: Option<usize>,
    },
    /// Evaluate the original body after all setters, never on an empty loop.
    Body,
    Finish,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyDecision {
    Collect,
    Continue,
    Finish,
    Propagate,
}

#[derive(Clone, Copy)]
enum LoopProgress {
    NotEntered,
    Entered,
    FinishedBeforeBody,
    FinishedAfterBody,
}

impl LoopProgress {
    fn finish(&mut self) {
        *self = match *self {
            Self::NotEntered | Self::FinishedBeforeBody => Self::FinishedBeforeBody,
            Self::Entered | Self::FinishedAfterBody => Self::FinishedAfterBody,
        };
    }

    fn finished(self) -> bool {
        matches!(self, Self::FinishedBeforeBody | Self::FinishedAfterBody)
    }

    fn entered(self) -> bool {
        matches!(self, Self::Entered | Self::FinishedAfterBody)
    }
}

/// Cursor state contains no variable identity, object copy, or body artifact.
/// Backends retain the original argv and native header/iterator receipts.
pub struct EachLoopState {
    recipe: NativeEachLoopRecipe,
    kind: NativeEachLoopKind,
    lengths: Vec<(usize, usize)>,
    cursors: Vec<usize>,
    iterations: usize,
    iteration: usize,
    group: usize,
    variable: usize,
    refresh: bool,
    beginning: bool,
    progress: LoopProgress,
    checking: usize,
    awaiting_check: bool,
}
impl EachLoopState {
    #[must_use]
    pub fn new(
        recipe: NativeEachLoopRecipe,
        kind: NativeEachLoopKind,
        lengths: Vec<(usize, usize)>,
    ) -> Self {
        let iterations = lengths
            .iter()
            .map(|&(variables, values)| {
                if variables == 0 {
                    0
                } else {
                    values.div_ceil(variables)
                }
            })
            .max()
            .unwrap_or(0);
        let cursors = vec![0; lengths.len()];
        Self {
            recipe,
            kind,
            lengths,
            cursors,
            iterations,
            iteration: 0,
            group: 0,
            variable: 0,
            refresh: true,
            beginning: true,
            progress: LoopProgress::NotEntered,
            checking: 0,
            awaiting_check: false,
        }
    }
    /// Update only the group lengths actually inspected by the selected backend.
    pub fn set_lengths(&mut self, group: usize, variables: usize, values: usize) {
        self.lengths[group] = (variables, values);
    }
    pub fn set_value_length(&mut self, group: usize, values: usize) {
        self.lengths[group].1 = values;
    }
    #[must_use]
    pub fn variable_cursor(&self) -> usize {
        self.variable
    }
    #[must_use]
    pub fn entered_body(&self) -> bool {
        self.progress.entered()
    }
    #[must_use]
    pub fn advance(&mut self) -> EachLoopAction {
        if self.progress.finished() {
            return EachLoopAction::Finish;
        }
        if self.beginning {
            if self.recipe.live_iterators() {
                if self.awaiting_check {
                    self.awaiting_check = false;
                    if self.cursors[self.checking] < self.lengths[self.checking].1 {
                        self.beginning = false;
                        self.checking = 0;
                    } else {
                        self.checking += 1;
                    }
                }
                if self.beginning {
                    if self.checking == self.lengths.len() {
                        self.progress.finish();
                        return EachLoopAction::Finish;
                    }
                    self.awaiting_check = true;
                    return EachLoopAction::Check(self.checking);
                }
            } else {
                self.beginning = false;
                if self.iteration >= self.iterations {
                    self.progress.finish();
                    return EachLoopAction::Finish;
                }
            }
        }
        while self.group < self.lengths.len() {
            if self.refresh {
                self.refresh = false;
                if self.recipe.refetches_groups() || self.recipe.live_iterators() {
                    return EachLoopAction::Refresh(self.group);
                }
            }
            let (variables, values) = self.lengths[self.group];
            if self.variable < variables {
                let variable = self.variable;
                let cursor = self.cursors[self.group];
                self.variable += 1;
                if cursor < values || !self.recipe.live_iterators() {
                    self.cursors[self.group] =
                        cursor.checked_add(1).expect("native loop cursor exhausted");
                }
                self.refresh = self.recipe.live_iterators();
                return EachLoopAction::Assign {
                    group: self.group,
                    variable,
                    value: (cursor < values).then_some(cursor),
                };
            }
            self.group += 1;
            self.variable = 0;
            self.refresh = true;
        }
        self.iteration += 1;
        self.group = 0;
        self.variable = 0;
        self.refresh = true;
        self.beginning = true;
        self.progress = LoopProgress::Entered;
        EachLoopAction::Body
    }
    #[must_use]
    pub fn body_completion(&mut self, code: Code) -> BodyDecision {
        match code {
            Code::Ok if self.kind == NativeEachLoopKind::Lmap => BodyDecision::Collect,
            Code::Ok | Code::Continue => BodyDecision::Continue,
            Code::Break => {
                self.progress.finish();
                BodyDecision::Finish
            }
            Code::Error | Code::Return | Code::Other(_) => BodyDecision::Propagate,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_schedule_defers_body_and_advances_before_callbacks() {
        let mut state = EachLoopState::new(
            NativeEachLoopRecipe::C(TclVersion::V8_5),
            NativeEachLoopKind::Foreach,
            vec![(2, 3)],
        );
        assert_eq!(
            state.advance(),
            EachLoopAction::Assign {
                group: 0,
                variable: 0,
                value: Some(0)
            }
        );
        assert_eq!(
            state.advance(),
            EachLoopAction::Assign {
                group: 0,
                variable: 1,
                value: Some(1)
            }
        );
        assert_eq!(state.advance(), EachLoopAction::Body);
        assert_eq!(
            state.body_completion(Code::Continue),
            BodyDecision::Continue
        );
        assert_eq!(
            state.advance(),
            EachLoopAction::Assign {
                group: 0,
                variable: 0,
                value: Some(2)
            }
        );
        assert_eq!(
            state.advance(),
            EachLoopAction::Assign {
                group: 0,
                variable: 1,
                value: None
            }
        );
        assert_eq!(state.advance(), EachLoopAction::Body);
        assert_eq!(state.body_completion(Code::Break), BodyDecision::Finish);
        assert_eq!(state.advance(), EachLoopAction::Finish);
        let mut empty = EachLoopState::new(
            NativeEachLoopRecipe::Jim084,
            NativeEachLoopKind::Lmap,
            vec![(1, 0)],
        );
        assert_eq!(empty.advance(), EachLoopAction::Check(0));
        empty.set_value_length(0, 0);
        assert_eq!(empty.advance(), EachLoopAction::Finish);
        assert!(!empty.entered_body());
    }
}

#[cfg(test)]
mod result_construction_tests {
    use super::*;
    use tcl_syntax::native_string::NativeStringProtocol;

    #[test]
    fn empty_variable_diagnostic_retains_only_its_selected_native_producer() {
        for version in TclVersion::ALL {
            let details = empty_variables(
                NativeEachLoopRecipe::C(version),
                NativeEachLoopKind::Foreach,
            )
            .into_byte_details();
            assert_eq!(
                details.string_result,
                Some(NativeStringProtocol::C(version))
            );
            assert_eq!(details.message, b"foreach varlist is empty");
        }
        assert_eq!(
            empty_variables(NativeEachLoopRecipe::Jim084, NativeEachLoopKind::Foreach)
                .into_byte_details()
                .string_result,
            None
        );
    }
}
