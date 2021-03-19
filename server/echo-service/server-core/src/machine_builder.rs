#![allow(dead_code)]
use super::*;
use machine_adpter::MachineAdapter;

/// Wrapper for a shared macine adapter
type SharedMachineAdapter<T> = Arc<MachineAdapter<T>>;

/// The MachineBuilder provides a default implementation for building a machine
/// from a model. It is used by the machine constructors to complete construction or
/// extending a machine.
pub trait MachineBuilder {
    type InstructionSet: MachineImpl;

    /// Create a machine with a bounded queue.
    fn bounded<T>(
        machine: T, capacity: usize,
    ) -> (
        SharedMachine<T>,
        ::smol::channel::Sender<Self::InstructionSet>,
        SharedMachineAdapter<Self::InstructionSet>,
    )
    where
        T: 'static + Machine<Self::InstructionSet>,
        <Self as MachineBuilder>::InstructionSet: Send,
    {
        let channel = ::smol::channel::bounded::<Self::InstructionSet>(capacity);
        Self::prepare_create(machine, channel)
    }

    /// Extend a created machine with an additional instruction set, with a bounded queue.
    fn extend_bounded<T>(
        machine: &Arc<T>, capacity: usize,
    ) -> (
        ::smol::channel::Sender<Self::InstructionSet>,
        SharedMachineAdapter<Self::InstructionSet>,
    )
    where
        T: 'static + Machine<Self::InstructionSet>,
        <Self as MachineBuilder>::InstructionSet: Send,
    {
        let channel = ::smol::channel::bounded::<Self::InstructionSet>(capacity);
        Self::prepare_extend(machine, channel)
    }

    /// Create a machine with an unbounded queue.
    fn unbounded<T>(
        machine: T,
    ) -> (
        SharedMachine<T>,
        ::smol::channel::Sender<Self::InstructionSet>,
        SharedMachineAdapter<Self::InstructionSet>,
    )
    where
        T: 'static + Machine<Self::InstructionSet>,
        <Self as MachineBuilder>::InstructionSet: Send,
    {
        let channel = ::smol::channel::unbounded::<Self::InstructionSet>();
        Self::prepare_create(machine, channel)
    }

    /// Extend a created machine with an additional instruction set, with a unbounded queue.
    fn extend_unbounded<T>(
        machine: &Arc<T>,
    ) -> (
        ::smol::channel::Sender<Self::InstructionSet>,
        SharedMachineAdapter<Self::InstructionSet>,
    )
    where
        T: 'static + Machine<Self::InstructionSet>,
        <Self as MachineBuilder>::InstructionSet: Send,
    {
        let channel = ::smol::channel::unbounded::<Self::InstructionSet>();
        Self::prepare_extend(machine, channel)
    }

    /// Prepare for creating a machine.
    fn prepare_create<T>(
        machine: T,
        channel: (
            ::smol::channel::Sender<Self::InstructionSet>,
            ::smol::channel::Receiver<Self::InstructionSet>,
        ),
    ) -> (
        SharedMachine<T>,
        ::smol::channel::Sender<Self::InstructionSet>,
        SharedMachineAdapter<Self::InstructionSet>,
    )
    where
        T: 'static + Machine<Self::InstructionSet>,
        <Self as MachineBuilder>::InstructionSet: Send,
    {
        let machine: SharedMachine<T> = Arc::new(machine);
        let (sender, adapter) = Self::prepare_adapter(&machine, channel);
        (machine, sender, adapter)
    }

    /// Prepare for extending a machine.
    fn prepare_extend<T>(
        machine: &Arc<T>,
        channel: (
            ::smol::channel::Sender<Self::InstructionSet>,
            ::smol::channel::Receiver<Self::InstructionSet>,
        ),
    ) -> (
        ::smol::channel::Sender<Self::InstructionSet>,
        SharedMachineAdapter<Self::InstructionSet>,
    )
    where
        T: 'static + Machine<Self::InstructionSet>,
        <Self as MachineBuilder>::InstructionSet: Send,
    {
        let (sender, adapter) = Self::prepare_adapter(machine, channel);
        (sender, adapter)
    }

    /// Prepare for creating a machine adapter.
    fn prepare_adapter<T>(
        machine: &SharedMachine<T>,
        channel: (
            ::smol::channel::Sender<Self::InstructionSet>,
            ::smol::channel::Receiver<Self::InstructionSet>,
        ),
    ) -> (
        ::smol::channel::Sender<Self::InstructionSet>,
        SharedMachineAdapter<Self::InstructionSet>,
    )
    where
        T: 'static + Machine<Self::InstructionSet>,
    {
        let machine = Arc::clone(machine) as Arc<dyn Machine<Self::InstructionSet>>;
        let executor = get_executor();
        Self::create_adapter(machine, channel, executor)
    }

    /// Create the adapter, which drives received instructions into the machine.
    fn create_adapter(
        machine: Arc<dyn Machine<Self::InstructionSet>>,
        channel: (
            ::smol::channel::Sender<Self::InstructionSet>,
            ::smol::channel::Receiver<Self::InstructionSet>,
        ),
        executor: Arc<::smol::Executor<'static>>,
    ) -> (
        ::smol::channel::Sender<Self::InstructionSet>,
        SharedMachineAdapter<Self::InstructionSet>,
    ) {
        let (s, r) = channel;
        let adapter = MachineAdapter::new(machine, executor, r);
        let adapter = adapter.start();
        (s, adapter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(dead_code)]
    #[derive(Debug, Clone)]
    pub enum Example {
        Red,
        Green,
        Yellow,
    }

    // Do all of the bindings MachineImpl would do
    impl MachineImpl for Example {
        type Adapter = Example;
        type InstructionSet = Example;
    }
    impl MachineBuilder for Example {
        type InstructionSet = Example;
    }

    #[test]
    fn test_bounded_construction() {
        // This is a fairly trivial smoke test. It is designed to catch changes that may
        // need to be propagated into the MachineImpl derive macro. The extensive set of
        // MachineBuilder tests can be found in machine-foundation/machine_adapter.rs
        //
        pub struct Alice {}
        impl Machine<Example> for Alice {
            fn receive(&self, _cmd: Example, _sender: &mut MachineSender) {}
        }
        let alice = Alice {};
        let (_alice, _sender, _adapter) = Example::bounded(alice, 10);
    }
}
