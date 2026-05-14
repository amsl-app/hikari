pub struct CompletedBubble {
    pub text: String,
    pub delta: String,
}

pub struct BubbleAccumulator {
    complete_message: String,
    current_bubble: String,
    bubble_offset: usize,
}

impl BubbleAccumulator {
    pub fn new() -> Self {
        Self {
            complete_message: String::new(),
            current_bubble: String::new(),
            bubble_offset: 0,
        }
    }

    pub fn push(&mut self, text: &str) -> Vec<CompletedBubble> {
        self.complete_message.push_str(text);
        self.current_bubble.push_str(text);

        let mut completed = Vec::new();
        while let Some(sep_pos) = self.current_bubble.find("---") {
            let bubble_text = self
                .current_bubble
                .get(..sep_pos)
                .expect("sep_pos from find() is a valid byte boundary")
                .to_string();
            let delta = self
                .current_bubble
                .get(self.bubble_offset..sep_pos)
                .expect("bubble_offset and sep_pos are valid byte boundaries")
                .to_string();
            let remainder = self
                .current_bubble
                .get(sep_pos + 3..)
                .expect("sep_pos + 3 is valid because --- is 3 ASCII bytes")
                .to_string();
            completed.push(CompletedBubble {
                text: bubble_text,
                delta,
            });
            self.current_bubble = remainder;
            self.bubble_offset = 0;
        }
        completed
    }

    pub fn current_bubble(&self) -> &str {
        &self.current_bubble
    }

    pub fn pending_delta(&self, buffer_size: usize) -> Option<&str> {
        if self.current_bubble.len() > self.bubble_offset.saturating_add(buffer_size) {
            self.current_bubble.get(self.bubble_offset..)
        } else {
            None
        }
    }

    pub fn advance_offset(&mut self) {
        self.bubble_offset = self.current_bubble.len();
    }

    pub fn finalize(self) -> (String, Option<(String, String)>) {
        let bubble = if self.current_bubble.is_empty() {
            None
        } else {
            let delta = self
                .current_bubble
                .get(self.bubble_offset..)
                .expect("bubble_offset is a valid byte boundary")
                .to_string();
            Some((self.current_bubble, delta))
        };
        (self.complete_message, bubble)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUF: usize = 5;

    #[test]
    fn no_separator_single_chunk() {
        let mut acc = BubbleAccumulator::new();
        let completed = acc.push("hello world");
        assert!(completed.is_empty());
        let (msg, bubble) = acc.finalize();
        assert_eq!(msg, "hello world");
        let (text, delta) = bubble.expect("should have a final bubble");
        assert_eq!(text, "hello world");
        assert_eq!(delta, "hello world");
    }

    #[test]
    fn single_separator_in_one_chunk() {
        let mut acc = BubbleAccumulator::new();
        let completed = acc.push("hello---world");
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].text, "hello");
        assert_eq!(completed[0].delta, "hello");

        let (msg, bubble) = acc.finalize();
        assert_eq!(msg, "hello---world");
        let (text, delta) = bubble.expect("should have a final bubble");
        assert_eq!(text, "world");
        assert_eq!(delta, "world");
    }

    #[test]
    fn separator_split_across_chunks() {
        let mut acc = BubbleAccumulator::new();
        assert!(acc.push("hel").is_empty());
        assert!(acc.push("lo--").is_empty());
        let completed = acc.push("-world");
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].text, "hello");
        assert_eq!(completed[0].delta, "hello");

        let (msg, bubble) = acc.finalize();
        assert_eq!(msg, "hello---world");
        let (text, delta) = bubble.expect("should have a final bubble");
        assert_eq!(text, "world");
        assert_eq!(delta, "world");
    }

    #[test]
    fn multiple_separators_in_one_chunk() {
        let mut acc = BubbleAccumulator::new();
        let completed = acc.push("a---b---c");
        assert_eq!(completed.len(), 2);
        assert_eq!(completed[0].text, "a");
        assert_eq!(completed[1].text, "b");

        let (_, bubble) = acc.finalize();
        assert_eq!(bubble.expect("final bubble").0, "c");
    }

    #[test]
    fn empty_first_bubble() {
        let mut acc = BubbleAccumulator::new();
        let completed = acc.push("---world");
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].text, "");
        assert_eq!(completed[0].delta, "");

        let (_, bubble) = acc.finalize();
        assert_eq!(bubble.expect("final bubble").0, "world");
    }

    #[test]
    fn empty_final_bubble() {
        let mut acc = BubbleAccumulator::new();
        let completed = acc.push("hello---");
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].text, "hello");

        let (msg, bubble) = acc.finalize();
        assert_eq!(msg, "hello---");
        assert!(bubble.is_none(), "trailing separator leaves an empty final bubble");
    }

    #[test]
    fn pending_delta_below_threshold() {
        let acc = BubbleAccumulator::new();
        assert!(acc.pending_delta(BUF).is_none());
    }

    #[test]
    fn pending_delta_above_threshold() {
        let mut acc = BubbleAccumulator::new();
        acc.push("123456");
        let delta = acc.pending_delta(BUF);
        assert_eq!(delta, Some("123456"));
    }

    #[test]
    fn advance_offset_suppresses_already_streamed() {
        let mut acc = BubbleAccumulator::new();
        acc.push("123456");
        assert!(acc.pending_delta(BUF).is_some());
        acc.advance_offset();
        assert!(
            acc.pending_delta(BUF).is_none(),
            "already streamed delta should not re-appear"
        );
        acc.push("78");
        assert!(
            acc.pending_delta(BUF).is_none(),
            "two new bytes is below buffer threshold"
        );
    }

    #[test]
    fn complete_message_accumulates_across_splits() {
        let mut acc = BubbleAccumulator::new();
        acc.push("a---b");
        let (msg, _) = acc.finalize();
        assert_eq!(msg, "a---b");
    }

    #[test]
    fn delta_only_contains_unstreamed_portion() {
        let mut acc = BubbleAccumulator::new();
        acc.push("123456789");
        acc.advance_offset();
        // Separator arrives after "XYZ"; bubble_offset is 9 so delta is only "XYZ"
        let completed = acc.push("XYZ---end");
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].text, "123456789XYZ");
        assert_eq!(completed[0].delta, "XYZ");
    }
}
