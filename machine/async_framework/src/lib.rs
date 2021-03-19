use async_trait::async_trait;
use smart_default::*;

mod test_message;
pub use test_message::*;

mod state_table;
pub use state_table::*;

mod forwarder;
pub use forwarder::*;

mod executor;
pub use executor::*;

mod machine;
pub use machine::*;

mod machine_adapter;
pub use machine_adapter::*;

mod daisy_chain;
pub use daisy_chain::*;

mod chaos_monkey;
pub use chaos_monkey::*;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
