//! Undo stack (S9.3).

#[derive(Debug, Clone)]
pub enum EditCommand {
    /// Full buffer snapshot (simple, reliable).
    ReplaceAll { samples: Vec<f32> },
}

#[derive(Debug, Default)]
pub struct UndoStack {
    stack: Vec<EditCommand>,
    max: usize,
}

impl UndoStack {
    pub fn new(max: usize) -> Self {
        Self {
            stack: Vec::new(),
            max: max.max(1),
        }
    }

    pub fn push(&mut self, cmd: EditCommand) {
        if self.stack.len() >= self.max {
            self.stack.remove(0);
        }
        self.stack.push(cmd);
    }

    pub fn pop(&mut self) -> Option<EditCommand> {
        self.stack.pop()
    }

    pub fn can_undo(&self) -> bool {
        !self.stack.is_empty()
    }
}
