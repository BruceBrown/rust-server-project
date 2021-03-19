use smart_default::*;

#[allow(unused_imports)]
use machine_foundation::{get_executor, machine, set_default_channel_max, Machine, MachineSender};
#[allow(unused_imports)]
use std::{
    sync::{atomic::AtomicUsize, Arc},
    time::{self, Duration, Instant},
};

// piggy-back on the example instruction sets
#[allow(unused_imports)]
use instruction_set::{ChaosMonkeyMutation, TestMessage, TestMessageReceiver, TestMessageSender};

mod forwarder;
pub use forwarder::Forwarder;

mod daisy_chain;
pub use daisy_chain::DaisyChainDriver;

mod chaos_monkey;
pub use chaos_monkey::ChaosMonkeyDriver;

/// The TestDriver trait is implemented by tests and benchmarks for testing various throughput scenrios.
pub trait TestDriver {
    /// Setup the test, initilizing and configuring machines.
    fn setup(&mut self);
    /// Teardown the test, cleaning everything up along the way.
    fn teardown(driver: Self);
    /// Run a single iteration of a test. A benchmark might call this 100s of times or more.
    fn run(&self);
}

/// The wait_for_notification function provides a common way to wait for a TestDriver::run() to complete.
pub fn wait_for_notification(receiver: &TestMessageReceiver, _messages: usize, _duration: Duration) -> Result<(), ()> {
    let start = std::time::SystemTime::now();
    let executor = get_executor();
    let r = receiver.clone();
    smol::future::block_on(executor.run(async move {
        if let Ok(_cmd) = r.recv().await {
            let elapsed = start.elapsed().unwrap();
            log::info!("completed in {:#?}", elapsed);
        } else {
            log::error!("receiver got error");
        }
    }));
    Ok(())
}

#[cfg(test)]
mod tests {}
