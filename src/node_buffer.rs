use crate::{arena::NodeIndex, node::Node};
use std::ops::{Index, IndexMut};

#[derive(Debug)]
pub struct NodeBuffer {
    nodes: Box<[Node]>,
    used: usize,
    half: usize,
}

impl NodeBuffer {
    pub fn new(cap: usize, half: usize) -> Self {
        Self {
            nodes: vec![Node::default(); cap].into(),
            used: 0,
            half,
        }
    }

    pub const fn capacity(&self) -> usize {
        self.nodes.len()
    }

    pub fn reset(&mut self) {
        self.used = 0;
    }

    pub const fn empty(&self) -> bool {
        self.used == 0
    }

    pub fn get_contiguous(&mut self, required_length: usize) -> Option<NodeIndex> {
        if self.used + required_length > self.capacity() {
            return None;
        }

        let start = self.used;
        self.used += required_length;

        Some(NodeIndex::new(self.half, start))
    }

    pub fn clear_references(&mut self) {
        for node in &mut self.nodes {
            if let Some(child) = node.first_child() {
                if child.half() != self.half {
                    node.remove_children();
                }
            }
        }
    }
}

impl Index<NodeIndex> for NodeBuffer {
    type Output = Node;

    fn index(&self, index: NodeIndex) -> &Self::Output {
        &self.nodes[index.idx()]
    }
}

impl IndexMut<NodeIndex> for NodeBuffer {
    fn index_mut(&mut self, index: NodeIndex) -> &mut Self::Output {
        &mut self.nodes[index.idx()]
    }
}
