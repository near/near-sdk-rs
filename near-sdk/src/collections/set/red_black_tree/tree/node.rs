use super::{EnvStorageKey, Direction};

use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct RedBlackNode<T> {
    pub color: u8,
    pub is_right_child: bool,
    pub key: EnvStorageKey,
    pub parent_key: Option<EnvStorageKey>,
    pub left_key: Option<EnvStorageKey>,
    pub right_key: Option<EnvStorageKey>,
    pub value: T
}

pub trait RedBlackNodeValue: Ord {
    type OrdValue: Ord;

    fn ord_value(&self) -> &Self::OrdValue;
}

// Need specialization in stable rust for this to work as intended
pub trait LocalOrd: Ord {}
impl<T: Ord> LocalOrd for T {}
impl<T: LocalOrd> RedBlackNodeValue for T {
    type OrdValue = Self;

    fn ord_value(&self) -> &Self::OrdValue {
        self
    }
}

// impl<T> RedBlackNodeValue for T {
//     type OrdValue = Self;

//     fn ord_value(&self) -> &Self::OrdValue {
//         self
//     }
// }

// impl RedBlackNodeValue for u64 {
//     type OrdValue = Self;

//     fn ord_value(&self) -> &Self::OrdValue {
//         self
//     }
// }


impl<T> Ord for RedBlackNode<T> 
where
    T: RedBlackNodeValue
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.value.ord_value().cmp(&other.value.ord_value())
    }
}

impl<T> PartialOrd for RedBlackNode<T> 
where
    T: RedBlackNodeValue
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.value.ord_value().cmp(&other.value.ord_value()))
    }
}

impl<T> PartialEq for RedBlackNode<T> 
where
    T: RedBlackNodeValue
{
    fn eq(&self, other: &Self) -> bool {
        self.value.ord_value() == other.value.ord_value()
    }
}

impl<T> Eq for RedBlackNode<T> 
where
    T: RedBlackNodeValue
{}

impl<T> RedBlackNode<T> {
    pub fn key(&self) -> &EnvStorageKey {
        &self.key
    }

    pub fn parent_node_key(&self) -> Option<&EnvStorageKey> {
        self.parent_key.as_ref()
    }

    pub fn left_node_key(&self) -> Option<&EnvStorageKey> {
        self.left_key.as_ref()
    }

    pub fn right_node_key(&self) -> Option<&EnvStorageKey> {
        self.right_key.as_ref()
    }

    pub fn is_root(&self) -> bool {
        self.parent_key.is_none()
    }

    pub fn is_black(&self) -> bool {
        self.color > 0
    }

    pub fn is_double_black(&self) -> bool {
        self.color == 2
    }

    pub fn is_red(&self) -> bool {
        self.color == 0
    }

    pub fn is_left_child(&self) -> bool {
        !self.is_right_child
    }

    pub fn is_right_child(&self) -> bool {
        self.is_right_child
    }

    pub fn child_direction(&self) -> Direction {
        use Direction::*;
        if self.is_right_child { Right } else { Left }
    }

    pub fn has_right_child(&self) -> bool {
        self.right_key.is_some()
    }

    pub fn has_left_child(&self) -> bool {
        self.left_key.is_some()
    }

    pub fn has_child(&self, direction: &Direction) -> bool {
        use Direction::*;
        match direction {
            Left => self.has_left_child(),
            Right => self.has_right_child()
        }
    }

    pub fn set_right_child(&mut self, node: Option<&mut RedBlackNode<T>>) {
        self.right_key = node.as_ref().map(|child_node| child_node.key().clone());
        if let Some(child_node) = node {
            child_node.parent_key = Some(self.key().clone());
            child_node.is_right_child = true;
        }
    }

    pub fn set_left_child(&mut self, node: Option<&mut RedBlackNode<T>>) {
        self.left_key = node.as_ref().map(|child_node| child_node.key().clone());
        if let Some(child_node) = node {
            child_node.parent_key = Some(self.key().clone());
            child_node.is_right_child = false;
        }
    }

    pub fn set_child(&mut self, node: Option<&mut RedBlackNode<T>>, direction: &Direction) {
        use Direction::*;
        match direction {
            Left => self.set_left_child(node),
            Right => self.set_right_child(node)
        }
    }
}