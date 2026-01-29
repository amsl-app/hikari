use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmEmbeddingChunk {
    pub content: String,
    pub pages: Vec<u32>,
}

impl LlmEmbeddingChunk {
    pub fn new<T: IntoIterator<Item = u32>>(content: String, pages: T) -> Self {
        Self {
            content,
            pages: pages.into_iter().collect(),
        }
    }
    pub fn push_sentence<T: IntoIterator<Item = u32>>(&mut self, sentence: &str, pages: T) {
        self.content.push(' ');
        self.content.push_str(sentence);
        self.pages.extend(pages);
        self.pages.dedup();
        self.pages.sort_unstable();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct LlmEmbeddingQueryResult {
    pub content: String,
    pub source: Source,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Source {
    pub name: String,
    pub link: String,
    pub pages: Vec<u32>,
}

impl Display for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = { self.name.trim().trim_matches('"').replace('\"', "").clone() };

        let mut page_string = String::new();
        if !self.pages.is_empty() {
            page_string.push_str(" p. ");

            for (i, page) in self.pages.iter().enumerate() {
                if i == self.pages.len() - 1 && i > 0 {
                    page_string.push_str(" & ");
                } else if i > 0 {
                    page_string.push_str(", ");
                }

                page_string.push_str(&page.to_string());
            }
        }

        write!(f, "{name}{page_string}")
    }
}
