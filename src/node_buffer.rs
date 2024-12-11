use crate::{arena::NodeIndex, node::Node};
use std::ops::{Index, IndexMut};

#[derive(Debug)]
pub struct NodeBuffer {
    nodes: Box<[Node]>,
    len: usize,
    half: usize,
}

impl NodeBuffer {
    pub fn new(cap: usize, half: usize) -> Self {
        Self {
            nodes: vec![Node::default(); cap].into(),
            len: 0,
            half,
        }
    }

    pub const fn capacity(&self) -> usize {
        self.nodes.len()
    }

    pub const fn remaining(&self) -> usize {
        self.capacity() - self.len
    }

    pub fn reset(&mut self) {
        self.len = 0;
    }

    pub fn get_contiguous(&mut self, required_length: usize) -> Option<NodeIndex> {
        if self.len + required_length > self.nodes.len() {
            return None;
        }

        let start = self.len;
        self.len += required_length;

        self.nodes[start..self.len].iter_mut().for_each(|n| n.clear());

        Some(NodeIndex::new(self.half, start))
    }

    pub fn clear_references(&mut self, half: usize) {
        for node in &mut self.nodes {
            if let Some(child) = node.first_child() {
                if child.half() == half {
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
