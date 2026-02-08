use serde::Serialize;

use crate::word_count;

pub const LIMIT: u32 = 128_000;
pub const ERROR_MESSAGE: &str = "exceeded 128000 word limit, dropped by input order";

#[derive(Debug, Clone, Serialize)]
pub struct LimitItem {
    pub word_count: u32,
    pub include: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub fn limit_items(contents: &[String]) -> Vec<LimitItem> {
    let mut running_total = 0u32;
    contents
        .iter()
        .map(|text| {
            let count = word_count::count_words(text);
            if running_total.saturating_add(count) <= LIMIT {
                running_total = running_total.saturating_add(count);
                LimitItem {
                    word_count: count,
                    include: true,
                    error: None,
                }
            } else {
                LimitItem {
                    word_count: count,
                    include: false,
                    error: Some(ERROR_MESSAGE.to_string()),
                }
            }
        })
        .collect()
}
