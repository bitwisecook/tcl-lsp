//! The debug controller: breakpoints + the step-mode decision logic.
//!
//! The VM backend calls [`DebugController::should_stop`] at each source-line
//! boundary with the current line and frame level; the controller decides
//! whether to pause (breakpoint hit, or the active step mode is satisfied). When
//! the frontend
//! resumes, it calls [`DebugController::resume`] with the next [`StepMode`],
//! which re-anchors the step depth to the frame the VM stopped in.
//!
//! This is the portable, VM-independent heart of the debugger and is fully
//! unit-tested; the threading/IO coordination lives in the backend.

use std::collections::BTreeSet;

use crate::types::{StepMode, StopReason};

/// Breakpoints + step-mode state.
#[derive(Debug)]
pub struct DebugController {
    breakpoints: BTreeSet<u32>,
    step_mode: StepMode,
    /// The frame level the current step was anchored at.
    step_depth: u32,
}

impl Default for DebugController {
    fn default() -> Self {
        Self {
            breakpoints: BTreeSet::new(),
            // Stop at the first line, like the Python controller.
            step_mode: StepMode::StepIn,
            step_depth: 0,
        }
    }
}

impl DebugController {
    /// A fresh controller that stops at program entry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a line breakpoint; returns the line (accepted as-is — line→PC
    /// validation is the backend's concern).
    pub fn add_breakpoint(&mut self, line: u32) -> u32 {
        self.breakpoints.insert(line);
        line
    }

    /// Remove a line breakpoint.
    pub fn remove_breakpoint(&mut self, line: u32) {
        self.breakpoints.remove(&line);
    }

    /// Replace the breakpoint set; returns the accepted lines, sorted.
    pub fn set_breakpoints(&mut self, lines: impl IntoIterator<Item = u32>) -> Vec<u32> {
        self.breakpoints = lines.into_iter().collect();
        self.breakpoints.iter().copied().collect()
    }

    /// The current breakpoints, sorted.
    #[must_use]
    pub fn breakpoints(&self) -> Vec<u32> {
        self.breakpoints.iter().copied().collect()
    }

    /// The active step mode.
    #[must_use]
    pub fn step_mode(&self) -> StepMode {
        self.step_mode
    }

    /// Decide whether to pause at a source-line boundary, given the line and
    /// the current frame `level`.'s
    /// stop decision.
    ///
    /// Returns `Some(reason)` to stop (and the reason to report), or `None` to
    /// keep running. On a stop, the caller should [`resume`](Self::resume) with
    /// the frontend's chosen mode before continuing.
    #[must_use]
    pub fn should_stop(&self, line: u32, level: u32) -> Option<StopReason> {
        if self.breakpoints.contains(&line) {
            return Some(StopReason::Breakpoint);
        }
        let step_hit = match self.step_mode {
            StepMode::StepIn => true,
            StepMode::StepOver => level <= self.step_depth,
            StepMode::StepOut => level < self.step_depth,
            StepMode::Continue => false,
        };
        step_hit.then_some(StopReason::Step)
    }

    /// Record that the VM has paused in a frame at `level` — anchors the step
    /// depth so a subsequent `StepOver`/`StepOut` measures against it.
    pub fn anchor(&mut self, level: u32) {
        self.step_depth = level;
    }

    /// Apply the frontend's resume decision: set the next step mode and
    /// re-anchor the depth to the frame just stopped in.
    pub fn resume(&mut self, mode: StepMode, level: u32) {
        self.step_mode = mode;
        self.step_depth = level;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_stops_at_first_line() {
        let c = DebugController::new();
        // Fresh controller is StepIn → stops at the first line in any frame.
        assert_eq!(c.should_stop(1, 0), Some(StopReason::Step));
    }

    #[test]
    fn breakpoint_stops_regardless_of_mode() {
        let mut c = DebugController::new();
        c.resume(StepMode::Continue, 0);
        c.add_breakpoint(5);
        assert_eq!(c.should_stop(5, 0), Some(StopReason::Breakpoint));
        assert_eq!(c.should_stop(6, 0), None);
    }

    #[test]
    fn continue_only_stops_on_breakpoints() {
        let mut c = DebugController::new();
        c.resume(StepMode::Continue, 0);
        assert_eq!(c.should_stop(1, 0), None);
        assert_eq!(c.should_stop(1, 3), None);
    }

    #[test]
    fn step_over_skips_deeper_frames() {
        let mut c = DebugController::new();
        // Stopped at level 1; step over.
        c.resume(StepMode::StepOver, 1);
        // A call descends to level 2 — do not stop.
        assert_eq!(c.should_stop(10, 2), None);
        // Back in the same frame — stop.
        assert_eq!(c.should_stop(11, 1), Some(StopReason::Step));
        // Returned to a shallower frame — also stop.
        assert_eq!(c.should_stop(12, 0), Some(StopReason::Step));
    }

    #[test]
    fn step_out_stops_only_when_shallower() {
        let mut c = DebugController::new();
        c.resume(StepMode::StepOut, 2);
        assert_eq!(c.should_stop(10, 2), None); // same frame
        assert_eq!(c.should_stop(10, 3), None); // deeper
        assert_eq!(c.should_stop(10, 1), Some(StopReason::Step)); // returned
    }

    #[test]
    fn breakpoint_set_roundtrips_sorted() {
        let mut c = DebugController::new();
        assert_eq!(c.set_breakpoints([7, 2, 5, 2]), vec![2, 5, 7]);
        c.remove_breakpoint(5);
        assert_eq!(c.breakpoints(), vec![2, 7]);
    }
}
