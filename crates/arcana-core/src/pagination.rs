//! Pagination types for list operations.

use serde::{Deserialize, Serialize};

/// A request for a page of results.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PageRequest {
    /// The page number (0-indexed).
    pub page: usize,
    /// The number of items per page.
    pub size: usize,
}

impl PageRequest {
    /// The default page size.
    pub const DEFAULT_SIZE: usize = 20;
    /// The maximum allowed page size.
    pub const MAX_SIZE: usize = 100;

    /// Creates a new page request.
    #[must_use]
    pub fn new(page: usize, size: usize) -> Self {
        Self {
            page,
            size: size.min(Self::MAX_SIZE),
        }
    }

    /// Creates a page request for the first page with default size.
    #[must_use]
    pub fn first() -> Self {
        Self::new(0, Self::DEFAULT_SIZE)
    }

    /// Returns the offset for database queries.
    #[must_use]
    pub const fn offset(&self) -> usize {
        self.page * self.size
    }

    /// Returns the limit for database queries.
    #[must_use]
    pub const fn limit(&self) -> usize {
        self.size
    }
}

impl Default for PageRequest {
    fn default() -> Self {
        Self::first()
    }
}

/// Information about a page of results.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PageInfo {
    /// The current page number (0-indexed).
    pub page: usize,
    /// The number of items per page.
    pub size: usize,
    /// The total number of items across all pages.
    pub total_elements: u64,
    /// The total number of pages.
    pub total_pages: u64,
    /// Whether this is the first page.
    pub first: bool,
    /// Whether this is the last page.
    pub last: bool,
    /// The number of items on this page.
    pub number_of_elements: usize,
}

impl PageInfo {
    /// Creates a new page info.
    #[must_use]
    pub fn new(page: usize, size: usize, total_elements: u64, number_of_elements: usize) -> Self {
        let total_pages = if size > 0 {
            (total_elements + size as u64 - 1) / size as u64
        } else {
            0
        };

        Self {
            page,
            size,
            total_elements,
            total_pages,
            first: page == 0,
            last: page as u64 >= total_pages.saturating_sub(1),
            number_of_elements,
        }
    }
}

/// A page of results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page<T> {
    /// The items on this page.
    pub content: Vec<T>,
    /// Information about this page.
    #[serde(flatten)]
    pub info: PageInfo,
}

impl<T> Page<T> {
    /// Creates a new page.
    #[must_use]
    pub fn new(content: Vec<T>, page: usize, size: usize, total_elements: u64) -> Self {
        let number_of_elements = content.len();
        Self {
            content,
            info: PageInfo::new(page, size, total_elements, number_of_elements),
        }
    }

    /// Creates an empty page.
    #[must_use]
    pub fn empty(page: usize, size: usize) -> Self {
        Self::new(Vec::new(), page, size, 0)
    }

    /// Maps the page content to a different type.
    #[must_use]
    pub fn map<U, F: FnMut(T) -> U>(self, f: F) -> Page<U> {
        Page {
            content: self.content.into_iter().map(f).collect(),
            info: self.info,
        }
    }

    /// Returns true if the page is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    /// Returns the number of items on this page.
    #[must_use]
    pub fn len(&self) -> usize {
        self.content.len()
    }

    /// Returns the total number of elements across all pages.
    #[must_use]
    pub const fn total_elements(&self) -> u64 {
        self.info.total_elements
    }

    /// Returns the total number of pages.
    #[must_use]
    pub const fn total_pages(&self) -> u64 {
        self.info.total_pages
    }

    /// Returns true if there is a next page.
    #[must_use]
    pub const fn has_next(&self) -> bool {
        !self.info.last
    }

    /// Returns true if there is a previous page.
    #[must_use]
    pub const fn has_previous(&self) -> bool {
        !self.info.first
    }
}

impl<T> Default for Page<T> {
    fn default() -> Self {
        Self::empty(0, PageRequest::DEFAULT_SIZE)
    }
}

impl<T> IntoIterator for Page<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.content.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_request() {
        let req = PageRequest::new(2, 10);
        assert_eq!(req.offset(), 20);
        assert_eq!(req.limit(), 10);
    }

    #[test]
    fn test_page_request_max_size() {
        let req = PageRequest::new(0, 1000);
        assert_eq!(req.size, PageRequest::MAX_SIZE);
    }

    #[test]
    fn test_page_request_first() {
        let req = PageRequest::first();
        assert_eq!(req.page, 0);
        assert_eq!(req.offset(), 0);
    }

    #[test]
    fn test_page_request_offset_calculation() {
        let req = PageRequest::new(0, 20);
        assert_eq!(req.offset(), 0);

        let req2 = PageRequest::new(1, 20);
        assert_eq!(req2.offset(), 20);

        let req3 = PageRequest::new(5, 15);
        assert_eq!(req3.offset(), 75);
    }

    #[test]
    fn test_page_info() {
        let page: Page<i32> = Page::new(vec![1, 2, 3], 0, 10, 25);
        assert!(page.info.first);
        assert!(!page.info.last);
        assert_eq!(page.info.total_pages, 3);
        assert!(page.has_next());
        assert!(!page.has_previous());
    }

    #[test]
    fn test_page_info_last_page() {
        let page: Page<i32> = Page::new(vec![1, 2], 2, 10, 22);
        assert!(!page.info.first);
        assert!(page.info.last);
        assert!(!page.has_next());
        assert!(page.has_previous());
    }

    #[test]
    fn test_page_map() {
        let page = Page::new(vec![1, 2, 3], 0, 10, 3);
        let mapped = page.map(|x| x * 2);
        assert_eq!(mapped.content, vec![2, 4, 6]);
    }

    #[test]
    fn test_page_empty() {
        let page: Page<i32> = Page::empty(0, 10);
        assert!(page.is_empty());
        assert_eq!(page.len(), 0);
        assert_eq!(page.total_elements(), 0);
        assert_eq!(page.total_pages(), 0);
    }

    #[test]
    fn test_page_is_not_empty() {
        let page = Page::new(vec![1, 2, 3], 0, 10, 3);
        assert!(!page.is_empty());
        assert_eq!(page.len(), 3);
    }

    #[test]
    fn test_page_total_elements_and_pages() {
        let page: Page<i32> = Page::new(vec![1], 0, 5, 11);
        assert_eq!(page.total_elements(), 11);
        assert_eq!(page.total_pages(), 3); // ceil(11/5) = 3
    }

    #[test]
    fn test_page_single_page() {
        let page = Page::new(vec![1, 2, 3], 0, 10, 3);
        assert!(page.info.first);
        assert!(page.info.last);
        assert!(!page.has_next());
        assert!(!page.has_previous());
    }
}
