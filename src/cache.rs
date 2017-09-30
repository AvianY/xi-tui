use xrl::{Line, Operation, OperationType, Update};

// use errors::*;

#[derive(Clone, Debug)]
pub struct LineCache {
    invalid_before: u64,
    pub lines: Vec<Line>,
    invalid_after: u64,
}

impl LineCache {
    pub fn new() -> Self {
        LineCache {
            invalid_before: 0,
            lines: vec![],
            invalid_after: 0,
        }
    }

    pub fn update(&mut self, update: Update) {
        let LineCache { ref mut lines, .. } = *self;
        let mut helper = UpdateHelper::new(lines);
        helper.update(update.operations);
        self.invalid_before = helper.invalid_before;
        self.invalid_after = helper.invalid_after;
    }
}

struct UpdateHelper<'a> {
    old_lines: &'a mut Vec<Line>,
    invalid_before: u64,
    new_lines: Vec<Line>,
    invalid_after: u64,
}

impl<'a> UpdateHelper<'a> {
    fn new(old_lines: &'a mut Vec<Line>) -> Self {
        UpdateHelper {
            old_lines: old_lines,
            invalid_before: 0,
            new_lines: Vec::new(),
            invalid_after: 0,
        }
    }

    fn get_fields_mut(&mut self) -> (&mut Vec<Line>, &mut Vec<Line>) {
        let UpdateHelper {
            ref mut old_lines,
            ref mut new_lines,
            ..
        } = *self;
        (old_lines, new_lines)
    }

    fn apply_copy(&mut self, nb_lines: u64) {
        let (old_lines, new_lines) = self.get_fields_mut();
        new_lines.extend(old_lines.drain(0..nb_lines as usize))
    }

    fn apply_skip(&mut self, nb_lines: u64) {
        let _ = self.old_lines.drain(0..nb_lines as usize).last();
    }

    fn apply_invalidate(&mut self, nb_lines: u64) {
        if self.new_lines.is_empty() {
            self.invalid_before = nb_lines;
        } else {
            self.invalid_after = nb_lines;
        }
    }

    fn apply_insert(&mut self, lines: Vec<Line>) {
        self.new_lines.extend(lines);
    }

    fn apply_update(&mut self, nb_lines: u64, lines: Vec<Line>) {
        let (old_lines, new_lines) = self.get_fields_mut();
        new_lines.extend(
            old_lines
                .drain(0..nb_lines as usize)
                .zip(lines.into_iter())
                .map(|(mut old_line, update)| {
                    old_line.cursor = update.cursor;
                    old_line.styles = update.styles;
                    old_line
                }),
        )
    }

    fn update(&mut self, operations: Vec<Operation>) {
        for op in operations {
            match op.operation_type {
                OperationType::Copy_ => self.apply_copy(op.nb_lines),
                OperationType::Skip => self.apply_skip(op.nb_lines),
                OperationType::Invalidate => self.apply_invalidate(op.nb_lines),
                OperationType::Insert => self.apply_insert(op.lines),
                OperationType::Update => self.apply_update(op.nb_lines, op.lines),
            }
        }
    }
}
