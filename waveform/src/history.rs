//! Undo / redo stack (S9.3).
//!
//! Edits are recorded as commands that carry the state to restore. Structural
//! edits (cut, paste, delete) record the whole buffer; in-place effects record
//! only the affected sample range, so a gain or fade on a long file does not
//! clone the buffer per edit.

use crate::selection::Selection;

#[derive(Debug, Clone)]
pub enum EditCommand {
    /// Buffer state before a structural edit that changed the length.
    ReplaceAll {
        samples: Vec<f32>,
        selection: Selection,
    },
    /// Sample range before an in-place edit that kept the buffer length.
    ReplaceRegion {
        start: usize,
        end: usize,
        samples: Vec<f32>,
        selection: Selection,
    },
}

impl EditCommand {
    /// Selection that was active when the edit was made.
    pub fn selection(&self) -> Selection {
        match self {
            EditCommand::ReplaceAll { selection, .. } => *selection,
            EditCommand::ReplaceRegion { selection, .. } => *selection,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct UndoStack {
    undo: Vec<EditCommand>,
    redo: Vec<EditCommand>,
    max: usize,
}

impl UndoStack {
    pub fn new(max: usize) -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            max: max.max(1),
        }
    }

    /// Record a new edit. Pending redo history is discarded, as in any editor.
    pub fn push(&mut self, cmd: EditCommand) {
        self.redo.clear();
        self.record_undo(cmd);
    }

    /// Append an undo entry without touching the redo stack (used while redoing).
    pub(crate) fn record_undo(&mut self, cmd: EditCommand) {
        if self.undo.len() >= self.max {
            self.undo.remove(0);
        }
        self.undo.push(cmd);
    }

    /// Append a redo entry without touching the undo stack (used while undoing).
    pub(crate) fn record_redo(&mut self, cmd: EditCommand) {
        if self.redo.len() >= self.max {
            self.redo.remove(0);
        }
        self.redo.push(cmd);
    }

    pub fn pop(&mut self) -> Option<EditCommand> {
        self.undo.pop()
    }

    pub fn pop_redo(&mut self) -> Option<EditCommand> {
        self.redo.pop()
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo_depth(&self) -> usize {
        self.undo.len()
    }

    pub fn redo_depth(&self) -> usize {
        self.redo.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(n: usize) -> EditCommand {
        EditCommand::ReplaceAll {
            samples: vec![n as f32],
            selection: Selection::region(0, 1),
        }
    }

    #[test]
    fn push_starts_a_new_branch() {
        let mut s = UndoStack::new(4);
        s.push(cmd(1));
        s.push(cmd(2));
        assert!(s.can_undo());
        assert!(!s.can_redo());
        assert_eq!(s.undo_depth(), 2);
        let back = s.pop().unwrap();
        s.record_redo(back);
        assert!(s.can_redo());
        s.push(cmd(3));
        assert!(!s.can_redo(), "a new edit discards redo");
    }

    #[test]
    fn history_is_bounded() {
        let mut s = UndoStack::new(2);
        for i in 0..5 {
            s.push(cmd(i));
        }
        assert_eq!(s.undo_depth(), 2);
    }

    #[test]
    fn selection_is_carried_by_the_command() {
        let c = EditCommand::ReplaceRegion {
            start: 3,
            end: 9,
            samples: vec![0.0; 6],
            selection: Selection::region(3, 9),
        };
        assert_eq!(c.selection(), Selection::region(3, 9));
    }
}
