// SPDX-License-Identifier: AGPL-3.0

//! Worklist for managing execution paths in symbolic execution

/// Worklist for depth-first search path exploration
///
/// Manages a stack of execution states to explore, using DFS strategy.
#[derive(Debug)]
pub struct Worklist<T> {
    /// Stack of execution states
    stack: Vec<T>,
    /// Count of completed paths
    pub completed_paths: usize,
}

impl<T> Worklist<T> {
    /// Create a new empty worklist
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            completed_paths: 0,
        }
    }

    /// Push an execution state onto the worklist
    pub fn push(&mut self, item: T) {
        self.stack.push(item);
    }

    /// Pop an execution state from the worklist (DFS - last in, first out)
    pub fn pop(&mut self) -> Option<T> {
        self.stack.pop()
    }

    /// Get the number of pending items in the worklist
    pub fn len(&self) -> usize {
        self.stack.len()
    }

    /// Check if the worklist is empty
    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    /// Increment the completed paths counter
    pub fn mark_completed(&mut self) {
        self.completed_paths += 1;
    }

    /// Get the total number of completed paths
    pub fn get_completed_count(&self) -> usize {
        self.completed_paths
    }

    /// Clear all pending items
    pub fn clear(&mut self) {
        self.stack.clear();
    }

    /// Get an iterator over the pending items (without consuming)
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.stack.iter()
    }
}

impl<T> Default for Worklist<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> std::ops::Index<usize> for Worklist<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.stack[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worklist_basic() {
        let mut worklist: Worklist<i32> = Worklist::new();

        assert!(worklist.is_empty());
        assert_eq!(worklist.len(), 0);

        worklist.push(1);
        worklist.push(2);
        worklist.push(3);

        assert_eq!(worklist.len(), 3);
        assert!(!worklist.is_empty());
    }

    #[test]
    fn test_worklist_dfs() {
        let mut worklist: Worklist<i32> = Worklist::new();

        worklist.push(1);
        worklist.push(2);
        worklist.push(3);

        // DFS: last in, first out
        assert_eq!(worklist.pop(), Some(3));
        assert_eq!(worklist.pop(), Some(2));
        assert_eq!(worklist.pop(), Some(1));
        assert_eq!(worklist.pop(), None);
    }

    #[test]
    fn test_worklist_completed_count() {
        let mut worklist: Worklist<i32> = Worklist::new();

        assert_eq!(worklist.get_completed_count(), 0);

        worklist.mark_completed();
        worklist.mark_completed();
        worklist.mark_completed();

        assert_eq!(worklist.get_completed_count(), 3);
    }

    #[test]
    fn test_worklist_clear() {
        let mut worklist: Worklist<i32> = Worklist::new();

        worklist.push(1);
        worklist.push(2);
        worklist.push(3);

        assert_eq!(worklist.len(), 3);

        worklist.clear();

        assert!(worklist.is_empty());
        assert_eq!(worklist.len(), 0);
    }

    #[test]
    fn test_worklist_index() {
        let mut worklist: Worklist<i32> = Worklist::new();

        worklist.push(10);
        worklist.push(20);
        worklist.push(30);

        assert_eq!(worklist[0], 10);
        assert_eq!(worklist[1], 20);
        assert_eq!(worklist[2], 30);
    }

    #[test]
    fn test_worklist_iter() {
        let mut worklist: Worklist<i32> = Worklist::new();

        worklist.push(1);
        worklist.push(2);
        worklist.push(3);

        let items: Vec<&i32> = worklist.iter().collect();
        assert_eq!(items, vec![&1, &2, &3]);
    }
}
