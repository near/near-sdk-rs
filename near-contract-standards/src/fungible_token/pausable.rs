
pub trait Pausable {
    fn pause(&mut self, p: bool);
    fn is_paused(&self) -> bool;
}
