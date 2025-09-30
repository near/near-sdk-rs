use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::{near, store::LazyOption, PanicOnDefault};

#[derive(BorshSerialize, BorshDeserialize, Ord, PartialOrd, Eq, PartialEq, Clone)]
#[borsh(crate = "near_sdk::borsh")]
pub struct Insertable {
    pub index: u32,
    pub data: String,
    pub is_valid: bool,
}

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct LazyContract {
    pub lazy_opt: LazyOption<Insertable>,
}

#[near]
impl LazyContract {
    #[init]
    pub fn new() -> Self {
        let lazy_opt = LazyOption::new(b"a", None);
        Self { lazy_opt }
    }

    fn insertable(&self) -> Insertable {
        Insertable { index: 0, data: "scatter cinnamon wheel useless please rough situate iron eager noise try evolve runway neglect onion".to_string(), is_valid: true }
    }

    /// This should only write to the underlying storage once.
    #[payable]
    pub fn flush(&mut self, iterations: usize) {
        let insertable = self.insertable();
        self.lazy_opt.set(Some(insertable));

        for _ in 0..=iterations {
            self.lazy_opt.flush();
        }
    }

    #[payable]
    pub fn get(&mut self, iterations: u32) {
        let insertable = self.insertable();
        self.lazy_opt.set(Some(insertable));
        for _ in 0..=iterations {
            self.lazy_opt.get();
        }
    }

    /// This should write on each iteration.
    #[payable]
    pub fn insert_flush(&mut self, iterations: u32) {
        let mut insertable = self.insertable();
        for idx in 0..=iterations {
            insertable.index = idx as u32;
            self.lazy_opt.set(Some(insertable.clone()));
            self.lazy_opt.flush();
        }
    }

    /// This should write twice on each iteration.
    #[payable]
    pub fn insert_take(&mut self, iterations: u32) {
        let mut insertable = self.insertable();
        for idx in 0..=iterations {
            insertable.index = idx as u32;
            self.lazy_opt.set(Some(insertable.clone()));
            self.lazy_opt.flush();
            self.lazy_opt.take();
            self.lazy_opt.flush();
        }
    }

    /// This should write and delete on each iteration.
    #[payable]
    pub fn insert_delete(&mut self, iterations: u32) {
        let insertable = self.insertable();
        for _ in 0..=iterations {
            self.lazy_opt.set(Some(insertable.clone()));
            self.lazy_opt.flush();
            self.lazy_opt.set(None);
            self.lazy_opt.flush();
        }
    }

    /// This should write once on each iteration.
    #[payable]
    pub fn insert_delete_flush_once(&mut self, iterations: u32) {
        let insertable = self.insertable();
        for _ in 0..=iterations {
            self.lazy_opt.set(Some(insertable.clone()));
            self.lazy_opt.set(None);
            self.lazy_opt.flush();
        }
    }
}
