use std::collections::{hash_map::Entry, HashMap};
use syn::Type;

pub mod abi_generator;
pub mod abi_visitor;

pub struct TypeRegistry {
    pub types: HashMap<Box<Type>, u32>,
    last: u32,
}

impl TypeRegistry {
    fn new() -> TypeRegistry {
        TypeRegistry { types: HashMap::new(), last: 0 }
    }

    fn register_type(&mut self, t: Box<Type>) -> u32 {
        match self.types.entry(t) {
            Entry::Occupied(occupied) => *occupied.get(),
            Entry::Vacant(vacant) => {
                let id = self.last;
                vacant.insert(id);
                self.last += 1;
                id
            }
        }
    }
}
