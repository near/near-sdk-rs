use super::{Vector, ERR_INCONSISTENT_STATE};
use crate::{env, IntoStorageKey};

use borsh::{BorshDeserialize, BorshSerialize};

use core::mem;

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct Index(u32);

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Bucket<T>
where
    T: BorshSerialize,
{
    next_vacant: Option<Index>,
    occupied_count: u32,
    elements: Vector<Container<T>>,
}

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub enum Container<T> {
    Empty { next_index: Option<Index> },
    Occupied(T),
}

impl<T> Container<T> {
    fn into_value(self) -> Option<T> {
        if let Container::Occupied(value) = self {
            Some(value)
        } else {
            None
        }
    }
}

// TODO debug impl (must be manual)

impl<T> Bucket<T>
where
    T: BorshSerialize,
{
    pub fn new<S: IntoStorageKey>(prefix: S) -> Self {
        Self { next_vacant: None, occupied_count: 0, elements: Vector::new(prefix) }
    }
    pub fn len(&self) -> u32 {
        self.occupied_count
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<T> Bucket<T>
where
    T: BorshSerialize + BorshDeserialize,
{
    pub fn get(&self, index: Index) -> Option<&T> {
        if let Container::Occupied(value) = self.elements.get(index.0)? {
            Some(value)
        } else {
            None
        }
    }
    pub fn get_mut(&mut self, index: Index) -> Option<&mut T> {
        if let Container::Occupied(value) = self.elements.get_mut(index.0)? {
            Some(value)
        } else {
            None
        }
    }
    pub fn insert(&mut self, value: T) -> Index {
        let new_value = Container::Occupied(value);
        let inserted_index;
        if let Some(Index(vacant)) = self.next_vacant {
            // There is a vacant cell, put new value in that position
            let prev = self.elements.replace(vacant, new_value);
            inserted_index = vacant;

            if let Container::Empty { next_index } = prev {
                // Update pointer on bucket to this next index
                self.next_vacant = next_index;
            } else {
                env::panic_str(ERR_INCONSISTENT_STATE)
            }
        } else {
            // No vacant cells, push and return index of pushed element
            self.elements.push(new_value);
            inserted_index = self.elements.len() - 1;
        }

        self.occupied_count += 1;
        Index(inserted_index)
    }

    pub fn remove(&mut self, index: Index) -> Option<T> {
        let entry = self.elements.get_mut(index.0)?;

        if matches!(entry, Container::Empty { .. }) {
            // Entry has already been cleared, return None
            return None;
        }

        // Take next pointer from bucket to attach to empty cell put in store
        let next_index = mem::take(&mut self.next_vacant);
        let prev = mem::replace(entry, Container::Empty { next_index });

        prev.into_value()
    }
}
