use crate::execution::error::LlmExecutionError;
use futures_core::future::BoxFuture;

type UpdateMessageFn =
    Box<dyn Fn(String, Option<i32>, bool) -> BoxFuture<'static, Result<i32, LlmExecutionError>> + Send>;

fn trailing_partial_sep(s: &str) -> usize {
    if s.ends_with("--") {
        2
    } else { usize::from(s.ends_with('-')) }
}

pub struct BubbleAccumulator {
    complete_content: String,
    current_bubble: String,
    current_id: Option<i32>,
    delta_offset: usize,
    update_message_fn: UpdateMessageFn,
}

impl BubbleAccumulator {
    pub fn new(
        update_message_fn: impl Fn(String, Option<i32>, bool) -> BoxFuture<'static, Result<i32, LlmExecutionError>>
        + Send
        + 'static,
    ) -> Self {
        Self {
            current_bubble: String::new(),
            complete_content: String::new(),
            current_id: None,
            delta_offset: 0,
            update_message_fn: Box::new(update_message_fn),
        }
    }

    pub async fn push(&mut self, text: &str) -> Result<Vec<(String, i32)>, LlmExecutionError> {
        self.current_bubble.push_str(text);
        self.complete_content.push_str(text);

        let mut bubble_chunks = Vec::new();

        if self.current_bubble.contains("---") {
            let mut last_part: Option<String> = None;
            for part in self.current_bubble.split("---") {
                if let Some(prev) = last_part.replace(part.to_string()) {
                    if !prev.is_empty() {
                        let id = self.update_message_fn.as_ref()(prev.clone(), self.current_id, true).await?;
                        if self.delta_offset < prev.len() {
                            let delta = prev
                                .get(self.delta_offset..)
                                .expect("delta_offset is a valid byte boundary")
                                .to_string();
                            bubble_chunks.push((delta, id));
                        }
                    }
                    self.current_id = None;
                    self.delta_offset = 0;
                }
            }
            if let Some(last) = last_part {
                self.current_bubble = last;
            }
        }

        // Hold back trailing "-" or "--" — they might be the start of a "---" separator.
        // Only save and stream content we're sure isn't part of an upcoming separator.
        let safe_end = self.current_bubble.len() - trailing_partial_sep(&self.current_bubble);
        if safe_end > self.delta_offset {
            let safe_content = self
                .current_bubble
                .get(..safe_end)
                .expect("safe_end is a valid byte boundary")
                .to_string();
            let id = self.update_message_fn.as_ref()(safe_content, self.current_id, false).await?;
            self.current_id = Some(id);

            let delta = self
                .current_bubble
                .get(self.delta_offset..safe_end)
                .expect("delta_offset and safe_end are valid byte boundaries")
                .to_string();
            self.delta_offset = safe_end;
            bubble_chunks.push((delta, id));
        }

        Ok(bubble_chunks)
    }

    pub async fn finalize(&mut self) -> Result<String, LlmExecutionError> {
        if !self.current_bubble.is_empty() {
            self.update_message_fn.as_ref()(self.current_bubble.clone(), self.current_id, true).await?;
        }
        Ok(self.complete_content.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    fn make_accumulator() -> (BubbleAccumulator, Arc<Mutex<Vec<(String, Option<i32>, bool)>>>) {
        let calls: Arc<Mutex<Vec<(String, Option<i32>, bool)>>> = Arc::new(Mutex::new(Vec::new()));
        let calls_clone = calls.clone();
        let counter = Arc::new(Mutex::new(0i32));
        let acc = BubbleAccumulator::new(move |content, id, done| {
            let calls = calls_clone.clone();
            let counter = counter.clone();
            Box::pin(async move {
                calls.lock().expect("calls lock").push((content, id, done));
                // Simulate upsert: return existing id on update, new id on insert
                if let Some(existing_id) = id {
                    return Ok(existing_id);
                }
                let mut c = counter.lock().expect("counter lock");
                *c += 1;
                Ok(*c)
            })
        });
        (acc, calls)
    }

    fn make_error_accumulator() -> BubbleAccumulator {
        BubbleAccumulator::new(|_, _, _| {
            Box::pin(async { Err(LlmExecutionError::Unexpected("test error".to_string())) })
        })
    }

    #[tokio::test]
    async fn push_single_chunk() {
        let (mut acc, calls) = make_accumulator();
        let chunks = acc.push("Hello").await.expect("push failed");
        assert_eq!(chunks, vec![("Hello".to_string(), 1)]);
        let calls = calls.lock().expect("lock");
        assert_eq!(calls.as_slice(), [("Hello".to_string(), None, false)]);
    }

    #[tokio::test]
    async fn push_incremental_deltas() {
        let (mut acc, calls) = make_accumulator();
        let first = acc.push("Hello").await.expect("push failed");
        assert_eq!(first, vec![("Hello".to_string(), 1)]);

        let second = acc.push(" World").await.expect("push failed");
        assert_eq!(second, vec![(" World".to_string(), 1)]);

        let calls = calls.lock().expect("lock");
        assert_eq!(
            calls.as_slice(),
            [
                ("Hello".to_string(), None, false),
                ("Hello World".to_string(), Some(1), false),
            ]
        );
    }

    #[tokio::test]
    async fn push_empty_returns_no_chunks() {
        let (mut acc, calls) = make_accumulator();
        let chunks = acc.push("").await.expect("push failed");
        assert!(chunks.is_empty());
        assert!(calls.lock().expect("lock").is_empty());
    }

    #[tokio::test]
    async fn push_separator_finalizes_bubble() {
        let (mut acc, calls) = make_accumulator();
        acc.push("Hello").await.expect("push failed");
        let chunks = acc.push("---").await.expect("push failed");

        // No delta for completed bubble (already streamed), new bubble not started yet
        assert!(chunks.is_empty());

        let calls = calls.lock().expect("lock");
        assert_eq!(
            calls.as_slice(),
            [("Hello".to_string(), None, false), ("Hello".to_string(), Some(1), true),]
        );
    }

    #[tokio::test]
    async fn push_separator_then_new_content() {
        let (mut acc, calls) = make_accumulator();
        acc.push("Hello").await.expect("push failed");
        acc.push("---").await.expect("push failed");
        let chunks = acc.push("World").await.expect("push failed");

        assert_eq!(chunks, vec![("World".to_string(), 2)]);

        let calls = calls.lock().expect("lock");
        assert_eq!(
            calls.as_slice(),
            [
                ("Hello".to_string(), None, false),
                ("Hello".to_string(), Some(1), true),
                ("World".to_string(), None, false),
            ]
        );
    }

    #[tokio::test]
    async fn push_inline_separator_splits_bubble() {
        let (mut acc, calls) = make_accumulator();
        let chunks = acc.push("Hello---World").await.expect("push failed");

        // "Hello" finalized, "World" is new in-progress bubble
        assert_eq!(chunks, vec![("Hello".to_string(), 1), ("World".to_string(), 2)]);

        let calls = calls.lock().expect("lock");
        assert_eq!(
            calls.as_slice(),
            [("Hello".to_string(), None, true), ("World".to_string(), None, false),]
        );
    }

    #[tokio::test]
    async fn push_multiple_separators() {
        let (mut acc, calls) = make_accumulator();
        let chunks = acc.push("A---B---C").await.expect("push failed");

        // "A" and "B" finalized, "C" in progress
        assert_eq!(
            chunks,
            vec![("A".to_string(), 1), ("B".to_string(), 2), ("C".to_string(), 3),]
        );

        let calls = calls.lock().expect("lock");
        assert_eq!(
            calls.as_slice(),
            [
                ("A".to_string(), None, true),
                ("B".to_string(), None, true),
                ("C".to_string(), None, false),
            ]
        );
    }

    #[tokio::test]
    async fn push_error_propagation() {
        let mut acc = make_error_accumulator();
        let result = acc.push("Hello").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn finalize_empty() {
        let (mut acc, calls) = make_accumulator();
        let content = acc.finalize().await.expect("finalize failed");
        assert_eq!(content, "");
        assert!(calls.lock().expect("lock").is_empty());
    }

    #[tokio::test]
    async fn finalize_persists_remaining_bubble() {
        let (mut acc, calls) = make_accumulator();
        acc.push("Hello").await.expect("push failed");
        let content = acc.finalize().await.expect("finalize failed");
        assert_eq!(content, "Hello");

        let calls = calls.lock().expect("lock");
        assert_eq!(
            calls.as_slice(),
            [("Hello".to_string(), None, false), ("Hello".to_string(), Some(1), true),]
        );
    }

    #[tokio::test]
    async fn finalize_complete_content_includes_separators() {
        let (mut acc, _calls) = make_accumulator();
        acc.push("Hello---World").await.expect("push failed");
        let content = acc.finalize().await.expect("finalize failed");
        assert_eq!(content, "Hello---World");
    }

    // --- split 1+2: first push ends with "-", second starts with "--"
    #[tokio::test]
    async fn push_separator_split_one_two() {
        let (mut acc, calls) = make_accumulator();
        // Trailing "-" is held back (might be start of "---"); only "Hello" is streamed
        let first = acc.push("Hello-").await.expect("push failed");
        assert_eq!(first, vec![("Hello".to_string(), 1)]);

        // Second push completes "---"; bubble finalized, new bubble starts
        let second = acc.push("--World").await.expect("push failed");
        assert_eq!(second, vec![("World".to_string(), 2)]);

        let calls = calls.lock().expect("lock");
        assert_eq!(
            calls.as_slice(),
            [
                // DB receives full "Hello-" (incl. partial sep) on every incremental save
                ("Hello".to_string(), None, false),
                // Finalized with content before "---"
                ("Hello".to_string(), Some(1), true),
                ("World".to_string(), None, false),
            ]
        );
    }

    // --- split 2+1: first push ends with "--", second starts with "-"
    #[tokio::test]
    async fn push_separator_split_two_one() {
        let (mut acc, calls) = make_accumulator();
        // Trailing "--" held back; only "Hello" is streamed
        let first = acc.push("Hello--").await.expect("push failed");
        assert_eq!(first, vec![("Hello".to_string(), 1)]);

        let second = acc.push("-World").await.expect("push failed");
        assert_eq!(second, vec![("World".to_string(), 2)]);

        let calls = calls.lock().expect("lock");
        assert_eq!(
            calls.as_slice(),
            [
                ("Hello".to_string(), None, false),
                ("Hello".to_string(), Some(1), true),
                ("World".to_string(), None, false),
            ]
        );
    }

    // separator split 1+2 with no leading content (separator starts the stream)
    #[tokio::test]
    async fn push_separator_split_one_two_at_start() {
        let (mut acc, calls) = make_accumulator();
        // Entire "-" held back; nothing emitted to client no update
        let first = acc.push("-").await.expect("push failed");
        assert!(first.is_empty());

        let second = acc.push("--World").await.expect("push failed");
        assert_eq!(second, vec![("World".to_string(), 1)]);

        let calls = calls.lock().expect("lock");
        assert_eq!(calls.as_slice(), [("World".to_string(), None, false),]);
    }

    // separator split 2+1 with no leading content (separator starts the stream)
    #[tokio::test]
    async fn push_separator_split_two_one_at_start() {
        let (mut acc, calls) = make_accumulator();
        // Entire "--" held back; nothing emitted to client
        let first = acc.push("--").await.expect("push failed");
        assert!(first.is_empty());

        let second = acc.push("-World").await.expect("push failed");
        assert_eq!(second, vec![("World".to_string(), 1)]);

        let calls = calls.lock().expect("lock");
        assert_eq!(calls.as_slice(), [("World".to_string(), None, false),]);
    }

    #[tokio::test]
    async fn push_separator_with_previous() {
        let (mut acc, calls) = make_accumulator();
        // Entire "--" held back; nothing emitted to client
        let first = acc.push("Text--").await.expect("push failed");
        assert_eq!(first, vec![("Text".to_string(), 1)]);

        let second = acc.push("-World").await.expect("push failed");
        assert_eq!(second, vec![("World".to_string(), 2)]);

        let calls = calls.lock().expect("lock");
        assert_eq!(
            calls.as_slice(),
            [
                ("Text".to_string(), None, false),
                ("Text".to_string(), Some(1), true),
                ("World".to_string(), None, false),
            ]
        );
    }

    // list items starting with "-" are not separators — they accumulate in one bubble
    #[tokio::test]
    async fn push_list_items() {
        let (mut acc, calls) = make_accumulator();
        let first = acc.push("-item").await.expect("push failed");
        assert_eq!(first, vec![("-item".to_string(), 1)]);

        // No separator between pushes — continues the same bubble
        let second = acc.push("-World").await.expect("push failed");
        assert_eq!(second, vec![("-World".to_string(), 1)]);

        let calls = calls.lock().expect("lock");
        assert_eq!(
            calls.as_slice(),
            [
                ("-item".to_string(), None, false),
                ("-item-World".to_string(), Some(1), false),
            ]
        );
    }

    // a single "-" at the end of a push is held back; released when next char is not "-"
    #[tokio::test]
    async fn push_list_items_end_with_separator() {
        let (mut acc, calls) = make_accumulator();
        // Trailing "-" in "-item\n-" held back; "\n" is not a separator guard
        let first = acc.push("-item\n-").await.expect("push failed");
        assert_eq!(first, vec![("-item\n".to_string(), 1)]);

        // "World" confirms the held "-" is content, not a separator
        let second = acc.push("World").await.expect("push failed");
        assert_eq!(second, vec![("-World".to_string(), 1)]);

        let calls = calls.lock().expect("lock");
        assert_eq!(
            calls.as_slice(),
            [
                ("-item\n".to_string(), None, false),
                ("-item\n-World".to_string(), Some(1), false),
            ]
        );
    }
}
