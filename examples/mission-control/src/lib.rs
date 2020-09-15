#[global_allocator]
static ALLOC: near_sdk::wee_alloc::WeeAlloc<'_> = near_sdk::wee_alloc::WeeAlloc::INIT;

mod account;
mod agent;
mod asset;
#[macro_use]
mod macros;
mod mission_control;
mod rate;
