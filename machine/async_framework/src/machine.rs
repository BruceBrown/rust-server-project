use super::*;
use crossbeam::atomic::AtomicCell;
use std::sync::Arc;

// These are the various flavors of machine constructors. A machine may be created with a default
// queue capacity, a specified queue capacity, or unbounded. If a machine supports additional
// instruction sets, it can be extended, again with a default, specified, or unbounded queue
// capacity.

/// The default bounded machine size.
#[allow(non_upper_case_globals)]
pub static default_channel_max: AtomicCell<usize> = AtomicCell::new(20);

/// Create a machine from a model with a default queue capacity. The Machine and Sender for the
/// machine are returned.
pub fn create<I, T>(
    machine: T,
) -> (
    SharedMachine<T>,
    ::smol::channel::Sender<<<I as MachineImpl>::Adapter as MachineBuilder>::InstructionSet>,
)
where
    T: 'static + Machine<I> + Machine<<<I as MachineImpl>::Adapter as MachineBuilder>::InstructionSet>,
    I: MachineImpl,
    <I as MachineImpl>::Adapter: MachineBuilder,
{
    let channel_max = default_channel_max.load();
    let (machine, sender, _adapter) = <<I as MachineImpl>::Adapter as MachineBuilder>::bounded(machine, channel_max);
    (machine, sender)
}

/// Create a machine from a model with a specified queue capacity. The Machine and Sender for the
/// machine are returned.
pub fn create_with_capacity<I, T>(
    machine: T, capacity: usize,
) -> (
    SharedMachine<T>,
    ::smol::channel::Sender<<<I as MachineImpl>::Adapter as MachineBuilder>::InstructionSet>,
)
where
    T: 'static + Machine<I> + Machine<<<I as MachineImpl>::Adapter as MachineBuilder>::InstructionSet>,
    I: MachineImpl,
    <I as MachineImpl>::Adapter: MachineBuilder,
{
    let (machine, sender, _adapter) = <<I as MachineImpl>::Adapter as MachineBuilder>::bounded(machine, capacity);
    (machine, sender)
}

/// Create a machine from a model with an unbounded queue capacity. The Machine and Sender for the
/// machine are returned.
pub fn create_unbounded<I, T>(
    machine: T,
) -> (
    SharedMachine<T>,
    ::smol::channel::Sender<<<I as MachineImpl>::Adapter as MachineBuilder>::InstructionSet>,
)
where
    T: 'static + Machine<I> + Machine<<<I as MachineImpl>::Adapter as MachineBuilder>::InstructionSet>,
    I: MachineImpl,
    <I as MachineImpl>::Adapter: MachineBuilder,
{
    let (machine, sender, _adapter) = <<I as MachineImpl>::Adapter as MachineBuilder>::unbounded(machine);
    (machine, sender)
}

/// Extend a machine with an additional instruction set and a default queue capacity. The Sender for the
/// machine is returned.
pub fn extend<I, T>(machine: &Arc<T>) -> ::smol::channel::Sender<<<I as MachineImpl>::Adapter as MachineBuilder>::InstructionSet>
where
    T: 'static + Machine<I> + Machine<<<I as MachineImpl>::Adapter as MachineBuilder>::InstructionSet>,
    I: MachineImpl,
    <I as MachineImpl>::Adapter: MachineBuilder,
{
    let channel_max = default_channel_max.load();
    let (sender, _adapter) = <<I as MachineImpl>::Adapter as MachineBuilder>::extend_bounded(machine, channel_max);
    sender
}

/// Extend a machine with an additional instruction set and a specified queue capacity. The Sender for the
/// machine is returned.
pub fn extend_with_capacity<I, T>(machine: &Arc<T>, capacity: usize) -> ::smol::channel::Sender<<<I as MachineImpl>::Adapter as MachineBuilder>::InstructionSet>
where
    T: 'static + Machine<I> + Machine<<<I as MachineImpl>::Adapter as MachineBuilder>::InstructionSet>,
    I: MachineImpl,
    <I as MachineImpl>::Adapter: MachineBuilder,
{
    let (sender, _adapter) = <<I as MachineImpl>::Adapter as MachineBuilder>::extend_bounded(machine, capacity);
    sender
}

/// Extend a machine with an additional instruction set and an unbounded queue capacity. The Sender for the
/// machine is returned.
pub fn extend_unbounded<I, T>(machine: &Arc<T>) -> ::smol::channel::Sender<<<I as MachineImpl>::Adapter as MachineBuilder>::InstructionSet>
where
    T: 'static + Machine<I> + Machine<<<I as MachineImpl>::Adapter as MachineBuilder>::InstructionSet>,
    I: MachineImpl,
    <I as MachineImpl>::Adapter: MachineBuilder,
{
    let (sender, _adapter) = <<I as MachineImpl>::Adapter as MachineBuilder>::extend_unbounded(machine);
    sender
}
