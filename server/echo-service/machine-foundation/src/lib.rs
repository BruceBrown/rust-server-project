mod executor;
pub mod machine;
mod machine_adapter;

pub use machine_adapter::{get_default_channel_max, set_default_channel_max};

pub use server_core::{
    get_default_num_threads, get_executor, set_default_num_threads, BackgroundTask, Machine, MachineBuilder, MachineImpl, MachineSender,
    SharedMachine,
};

#[cfg(test)]
mod tests {}
