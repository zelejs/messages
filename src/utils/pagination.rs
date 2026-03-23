use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct PaginationParams {
    #[serde(default = "default_page")]
    pub page: i64,

    #[serde(default = "default_page_size")]
    pub page_size: i64,
}

#[allow(dead_code)]
fn default_page() -> i64 {
    1
}

#[allow(dead_code)]
fn default_page_size() -> i64 {
    20
}

#[derive(Debug, Clone, Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub total_pages: i64,
}

impl<T> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, total: i64, page: i64, page_size: i64) -> Self {
        let total_pages = if page_size > 0 {
            (total + page_size - 1) / page_size
        } else {
            0
        };

        Self {
            data,
            total,
            page,
            page_size,
            total_pages,
        }
    }
}
