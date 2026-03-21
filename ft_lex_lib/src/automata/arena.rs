use std::fmt;

/// Index into an arena. Lightweight handle — Copy, Hash, Ord.
/// Used for both NFA and DFA state identifiers.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(pub usize);

impl fmt::Debug for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "N{}", self.0)
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Generic arena allocator for graph nodes.
/// Nodes are stored contiguously in a `Vec` and referenced by `NodeId`.
/// This avoids `Box`/`Rc` ownership issues with cyclic graphs.
pub struct Arena<T> {
    nodes: Vec<T>,
}

impl<T> Arena<T> {
    /// Create a new empty arena.
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    /// Create an arena with pre-allocated capacity.
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            nodes: Vec::with_capacity(cap),
        }
    }

    /// Allocate a new node, returning its id.
    pub fn alloc(&mut self, val: T) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(val);
        id
    }

    /// Get an immutable reference to a node.
    pub fn get(&self, id: NodeId) -> &T {
        &self.nodes[id.0]
    }

    /// Get a mutable reference to a node.
    pub fn get_mut(&mut self, id: NodeId) -> &mut T {
        &mut self.nodes[id.0]
    }

    /// Number of nodes in the arena.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether the arena is empty.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Iterate over all (NodeId, &T) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (NodeId, &T)> {
        self.nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (NodeId(i), n))
    }

    /// Iterate over all (NodeId, &mut T) pairs.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (NodeId, &mut T)> {
        self.nodes
            .iter_mut()
            .enumerate()
            .map(|(i, n)| (NodeId(i), n))
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: fmt::Debug> fmt::Debug for Arena<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Arena")
            .field("len", &self.nodes.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alloc_and_get() {
        let mut arena: Arena<String> = Arena::new();
        let id0 = arena.alloc("hello".to_string());
        let id1 = arena.alloc("world".to_string());
        assert_eq!(id0, NodeId(0));
        assert_eq!(id1, NodeId(1));
        assert_eq!(arena.get(id0), "hello");
        assert_eq!(arena.get(id1), "world");
    }

    #[test]
    fn test_get_mut() {
        let mut arena: Arena<i32> = Arena::new();
        let id = arena.alloc(42);
        *arena.get_mut(id) = 100;
        assert_eq!(*arena.get(id), 100);
    }

    #[test]
    fn test_len_and_empty() {
        let mut arena: Arena<u8> = Arena::new();
        assert!(arena.is_empty());
        assert_eq!(arena.len(), 0);
        arena.alloc(1);
        assert!(!arena.is_empty());
        assert_eq!(arena.len(), 1);
    }

    #[test]
    fn test_iter() {
        let mut arena: Arena<&str> = Arena::new();
        arena.alloc("a");
        arena.alloc("b");
        arena.alloc("c");
        let pairs: Vec<_> = arena.iter().collect();
        assert_eq!(pairs.len(), 3);
        assert_eq!(pairs[0], (NodeId(0), &"a"));
        assert_eq!(pairs[2], (NodeId(2), &"c"));
    }

    #[test]
    fn test_node_id_ordering() {
        let a = NodeId(0);
        let b = NodeId(1);
        let c = NodeId(0);
        assert!(a < b);
        assert_eq!(a, c);
        assert_ne!(a, b);
    }

    #[test]
    fn test_node_id_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(NodeId(0));
        set.insert(NodeId(1));
        set.insert(NodeId(0)); // duplicate
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_node_id_display() {
        assert_eq!(format!("{}", NodeId(42)), "42");
        assert_eq!(format!("{:?}", NodeId(42)), "N42");
    }

    #[test]
    fn test_with_capacity() {
        let arena: Arena<u8> = Arena::with_capacity(100);
        assert!(arena.is_empty());
    }
}
