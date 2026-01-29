use std::sync::Arc;

use indexmap::IndexMap;
use tokio::sync::Mutex;

use crate::builder::LlmStructureBuilder;
use crate::execution::error::LlmExecutionError;

use super::steps::LlmStep;

type StepMap = IndexMap<String, Arc<Mutex<LlmStep>>>;
pub struct LlmStepIterator {
    steps: StepMap,
    next_step: usize,
}

impl LlmStepIterator {
    pub fn new(action: LlmStructureBuilder, previous_next_step: Option<String>) -> Result<Self, LlmExecutionError> {
        let steps = action.build()?;
        let mut next_step = 0;
        if let Some(step) = previous_next_step {
            next_step = steps.iter().position(|(id, _)| id == &step).unwrap_or(0);
        }
        Ok(Self { steps, next_step })
    }
    pub fn goto(&mut self, step: &str) -> Result<(), LlmExecutionError> {
        self.next_step = self.steps.get_index_of(step).ok_or(LlmExecutionError::NoAction)?;
        Ok(())
    }

    #[must_use]
    pub fn get_step(&self, step: &str) -> Option<Arc<Mutex<LlmStep>>> {
        self.steps.get(step).cloned()
    }
}

impl Iterator for LlmStepIterator {
    type Item = Arc<Mutex<LlmStep>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_step < self.steps.len() {
            let action = self.steps.get_index(self.next_step).map(|(_, action)| action);
            self.next_step += 1;
            action.cloned()
        } else {
            None
        }
    }
}
