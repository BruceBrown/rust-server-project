#[macro_use] extern crate smart_default;

use rand::distributions::{Distribution, Uniform};
use rand::prelude::*;

mod test_message;
pub use test_message::*;

mod forwarder;
pub use forwarder::*;

mod machine_adapter;
pub use machine_adapter::*;

mod daisy_chain;
pub use daisy_chain::*;

mod chaos_monkey;
pub use chaos_monkey::*;

fn main() {
    println!("Hello, world!");
}
