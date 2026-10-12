// SPDX-License-Identifier: AGPL-3.0-or-later
//! Timer storage over original port-owned script objects, without name conversion.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// The actual registered kind, independently of source or script shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    /// A deadline-driven event.
    Timer,
    /// An idle handler.
    Idle,
}
/// One native event-service turn; callbacks may cancel still-live later entries.
#[derive(Clone, Copy)]
pub struct EventTurn {
    now: Instant,
    last_id: u64,
    kind: EventKind,
}
struct Timer<S> {
    id: u64,
    deadline: Instant,
    reported: EventKind,
    script: S,
}
/// Shared timer ordering and storage. The script handle is never parsed here.
pub struct EventQueue<S> {
    timers: Vec<Timer<S>>,
    idle: VecDeque<(u64, S)>,
    next_id: Option<u64>,
}
impl<S> Default for EventQueue<S> {
    fn default() -> Self {
        Self {
            timers: Vec::new(),
            idle: VecDeque::new(),
            next_id: None,
        }
    }
}
impl<S> EventQueue<S> {
    /// Register an original script under the selected first-id and kind rules.
    pub fn push(
        &mut self,
        first_id: u64,
        delay: Duration,
        kind: EventKind,
        reported: EventKind,
        script: S,
    ) -> u64 {
        let id = self.next_id.unwrap_or(first_id);
        self.next_id = Some(id + 1);
        match kind {
            EventKind::Timer => self.timers.push(Timer {
                id,
                deadline: Instant::now() + delay,
                reported,
                script,
            }),
            EventKind::Idle => self.idle.push_back((id, script)),
        }
        id
    }
    /// Remove the exact live event and return its original script handle.
    pub fn cancel(&mut self, id: u64) -> Option<S> {
        if let Some(i) = self.timers.iter().position(|t| t.id == id) {
            return Some(self.timers.remove(i).script);
        }
        self.idle
            .iter()
            .position(|(i, _)| *i == id)
            .and_then(|i| self.idle.remove(i).map(|(_, s)| s))
    }
    /// Remaining deadline of a current time event, before releasing its original script.
    #[must_use]
    pub fn remaining(&self, id: u64) -> Option<Duration> {
        self.timers
            .iter()
            .find(|timer| timer.id == id)
            .map(|timer| timer.deadline.saturating_duration_since(Instant::now()))
    }
    /// Observe an original retained script; its getters remain port-owned.
    #[must_use]
    pub fn script(&self, id: u64) -> Option<(&S, EventKind)> {
        self.timers
            .iter()
            .find(|t| t.id == id)
            .map(|t| (&t.script, t.reported))
            .or_else(|| {
                self.idle
                    .iter()
                    .find(|(i, _)| *i == id)
                    .map(|(_, s)| (s, EventKind::Idle))
            })
    }
    /// Current identifiers in selected deadline order or newest registration first.
    #[must_use]
    pub fn ids(&self, deadline_order: bool) -> Vec<u64> {
        if deadline_order {
            let mut timers: Vec<_> = self.timers.iter().collect();
            timers.sort_unstable_by_key(|t| (t.deadline, t.id));
            return timers.into_iter().map(|t| t.id).collect();
        }
        let mut ids: Vec<_> = self
            .timers
            .iter()
            .map(|t| t.id)
            .chain(self.idle.iter().map(|(id, _)| *id))
            .collect();
        ids.sort_unstable_by(|a, b| b.cmp(a));
        ids
    }
    /// Whether a wait has any timer or idle event source.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.timers.is_empty() && self.idle.is_empty()
    }
    /// Earliest pending deadline, without executing or generating script text.
    #[must_use]
    pub fn earliest_deadline(&self) -> Option<Instant> {
        self.timers.iter().map(|t| t.deadline).min()
    }
    /// Select the current due timer batch or idle generation without removing scripts.
    #[must_use]
    pub fn begin_turn(&self, now: Instant, idle_only: bool) -> Option<EventTurn> {
        let kind = if !idle_only && self.timers.iter().any(|t| t.deadline <= now) {
            EventKind::Timer
        } else if !self.idle.is_empty() {
            EventKind::Idle
        } else {
            return None;
        };
        Some(EventTurn {
            now,
            last_id: self.next_id?.saturating_sub(1),
            kind,
        })
    }
    /// Remove the next still-live entry in the selected turn. Later registrations
    /// belong to a new turn, and a cancelled entry cannot execute from a copied batch.
    pub fn pop_turn(&mut self, turn: EventTurn) -> Option<S> {
        match turn.kind {
            EventKind::Timer => {
                let next = self
                    .timers
                    .iter()
                    .enumerate()
                    .filter(|(_, t)| t.id <= turn.last_id && t.deadline <= turn.now)
                    .min_by_key(|(_, t)| (t.deadline, t.id))
                    .map(|(i, _)| i)?;
                Some(self.timers.remove(next).script)
            }
            EventKind::Idle => {
                if self.idle.front().is_some_and(|(id, _)| *id <= turn.last_id) {
                    self.idle.pop_front().map(|(_, script)| script)
                } else {
                    None
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn current_event_turn_preserves_original_scripts_and_live_cancellation() {
        // Implementation proof: naming.event.original-queue-and-port-transport
        // docs/design/analysis/name-resolution-proofs/event-original-queue-and-port-transport.md
        use std::rc::Rc;
        let first = Rc::new(vec![0, 255]);
        let second = Rc::new(vec![192, 128]);
        let mut queue = EventQueue::default();
        assert_eq!(
            queue.push(
                0,
                Duration::ZERO,
                EventKind::Timer,
                EventKind::Timer,
                Rc::clone(&first)
            ),
            0
        );
        let id = queue.push(
            0,
            Duration::ZERO,
            EventKind::Timer,
            EventKind::Timer,
            Rc::clone(&second),
        );
        let turn = queue.begin_turn(Instant::now(), false).unwrap();
        assert!(Rc::ptr_eq(&queue.pop_turn(turn).unwrap(), &first));
        assert!(Rc::ptr_eq(&queue.cancel(id).unwrap(), &second));
        queue.push(
            0,
            Duration::ZERO,
            EventKind::Timer,
            EventKind::Timer,
            Rc::clone(&second),
        );
        assert!(queue.pop_turn(turn).is_none());
        let next = queue.begin_turn(Instant::now(), false).unwrap();
        assert!(Rc::ptr_eq(&queue.pop_turn(next).unwrap(), &second));
        assert!(queue.is_empty());
    }
}
