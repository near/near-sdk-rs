#[derive(Clone)]
#[cfg_attr(not(feature = "expensive-debug"), derive(Debug))]
pub(crate) struct CacheEntry<T> {
    value: Option<T>,
    state: EntryState,
}

impl<T> CacheEntry<T> {
    pub fn new(value: Option<T>, state: EntryState) -> Self {
        Self { value, state }
    }

    pub fn new_cached(value: Option<T>) -> Self {
        Self::new(value, EntryState::Cached)
    }

    pub fn new_modified(value: Option<T>) -> Self {
        Self::new(value, EntryState::Modified)
    }

    pub fn value(&self) -> &Option<T> {
        &self.value
    }

    pub fn value_mut(&mut self) -> &mut Option<T> {
        self.state = EntryState::Modified;
        &mut self.value
    }

    pub fn into_value(self) -> Option<T> {
        self.value
    }

    /// Replaces the current value with a new one. This changes the state of the cell to mutated
    /// if either the old or new value is [`Some<T>`].
    pub fn replace(&mut self, value: Option<T>) -> Option<T> {
        let old_value = core::mem::replace(&mut self.value, value);

        if self.value.is_some() || old_value.is_some() {
            // Set modified if both values are not `None`
            self.state = EntryState::Modified;
        }

        old_value
    }
}

#[derive(Copy, Clone)]
#[cfg_attr(not(feature = "expensive-debug"), derive(Debug))]
pub(crate) enum EntryState {
    Modified,
    Cached,
}

impl EntryState {
    /// Returns true if the entry has been modified
    pub fn is_modified(self) -> bool {
        matches!(self, EntryState::Cached)
    }

    /// Returns true if the entry state has not been changed.
    pub fn is_cached(self) -> bool {
        !self.is_modified()
    }
}
