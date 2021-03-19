use super::*;

use std::{fmt, sync::Arc};
use uuid::Uuid;

pub type SharedMachineAdapter<T> = Arc<MachineAdapter<T>>;
pub type SharedMachine<T> = Arc<T>;

/// The MachineImpl binds an adapter and instruction set together via an enum variant.
pub trait MachineImpl: 'static + Send + Sync {
    type Adapter;
    type InstructionSet: Send + Sync + Clone;
}

// The SendContext contains a Sender and Instruction. Its used by the MachineSender.
struct SendContext<T: MachineImpl>(::smol::channel::Sender<T>, Option<T>);

// The AsyncSender trait is stackable, and exposes an async fn for sending an instruction to a sender.
#[async_trait]
trait AsyncSender: Send + Sync {
    async fn do_send(&mut self);
}

// The implementation of SendContext, which erases the generic type T.
#[async_trait]
impl<T> AsyncSender for SendContext<T>
where
    T: MachineImpl,
{
    async fn do_send(&mut self) { self.0.send(self.1.take().unwrap()).await.ok(); }
}

/// The Machine trait must be implemented by each machine model for each instruction set that
/// it supports.
pub trait Machine<T>: Send + Sync
where
    T: 'static + Send + Sync,
{
    fn receive(&self, cmd: T, sender: &mut MachineSender);
    fn disconnected(&self) {}
    fn connected(&self, _uuid: uuid::Uuid) {}
}

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
    fn extend_bounded<T>(machine: &Arc<T>, capacity: usize) -> (::smol::channel::Sender<Self::InstructionSet>, SharedMachineAdapter<Self::InstructionSet>)
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
    fn extend_unbounded<T>(machine: &Arc<T>) -> (::smol::channel::Sender<Self::InstructionSet>, SharedMachineAdapter<Self::InstructionSet>)
    where
        T: 'static + Machine<Self::InstructionSet>,
        <Self as MachineBuilder>::InstructionSet: Send,
    {
        let channel = ::smol::channel::unbounded::<Self::InstructionSet>();
        Self::prepare_extend(machine, channel)
    }

    /// Prepare for creating a machine.
    fn prepare_create<T>(
        machine: T, channel: (::smol::channel::Sender<Self::InstructionSet>, ::smol::channel::Receiver<Self::InstructionSet>),
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
        machine: &Arc<T>, channel: (::smol::channel::Sender<Self::InstructionSet>, ::smol::channel::Receiver<Self::InstructionSet>),
    ) -> (::smol::channel::Sender<Self::InstructionSet>, SharedMachineAdapter<Self::InstructionSet>)
    where
        T: 'static + Machine<Self::InstructionSet>,
        <Self as MachineBuilder>::InstructionSet: Send,
    {
        let (sender, adapter) = Self::prepare_adapter(machine, channel);
        (sender, adapter)
    }

    /// Prepare for creating a machine adapter.
    fn prepare_adapter<T>(
        machine: &SharedMachine<T>, channel: (::smol::channel::Sender<Self::InstructionSet>, ::smol::channel::Receiver<Self::InstructionSet>),
    ) -> (::smol::channel::Sender<Self::InstructionSet>, SharedMachineAdapter<Self::InstructionSet>)
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
        channel: (::smol::channel::Sender<Self::InstructionSet>, ::smol::channel::Receiver<Self::InstructionSet>), executor: Arc<::smol::Executor<'static>>,
    ) -> (::smol::channel::Sender<Self::InstructionSet>, SharedMachineAdapter<Self::InstructionSet>) {
        let (s, r) = channel;
        let adapter = MachineAdapter::new(machine, executor, r);
        let adapter = adapter.start();
        (s, adapter)
    }
}

/// The MachineSender object is opaque, exposing a single send method. It is used by the receiver to send
/// instructions to other machines.
#[derive(Default)]
pub struct MachineSender {
    queue: Vec<Box<dyn AsyncSender>>,
}
impl MachineSender {
    /// Send an instruction to another machine.
    pub fn send<T: MachineImpl>(&mut self, sender: ::smol::channel::Sender<T>, cmd: T) {
        let sender = Box::new(SendContext(sender, Some(cmd))) as Box<dyn AsyncSender>;
        self.queue.push(sender);
    }
}

/// The MachineAdapter binds the machine, its receiver, and an executor together.
pub struct MachineAdapter<T: MachineImpl> {
    id: Uuid,
    pub machine: Arc<dyn Machine<T>>,
    pub executor: Arc<::smol::Executor<'static>>,
    pub receiver: smol::channel::Receiver<T>,
}

impl<T: MachineImpl> std::fmt::Debug for MachineAdapter<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "#MachineAdapter {{ .. }}") }
}

impl<T: MachineImpl> MachineAdapter<T> {
    // Construct a new MachineAdpter from its components.
    fn new(machine: Arc<dyn Machine<T>>, executor: Arc<::smol::Executor<'static>>, receiver: ::smol::channel::Receiver<T>) -> Self {
        let id = Uuid::new_v4();
        Self {
            id,
            machine,
            executor,
            receiver,
        }
    }

    // Start a Machine running. Once started, it runs until its receiver is closed.
    fn start(self) -> Arc<MachineAdapter<T>> {
        let r = self.receiver.clone();
        let machine = self.machine.clone();
        let id = self.id;
        let adapter = Arc::new(self);
        adapter
            .executor
            .spawn(async move {
                machine.connected(id);
                let mut sender = MachineSender::default();
                while let Ok(cmd) = r.recv().await {
                    sender.queue.clear();
                    machine.receive(cmd, &mut sender);
                    for s in sender.queue.iter_mut() {
                        s.do_send().await;
                    }
                }
                machine.disconnected();
            })
            .detach();
        adapter
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam::atomic::AtomicCell;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::thread;

    // A trivial machine
    #[derive(Debug)]
    struct Alice {
        uuid: AtomicCell<Uuid>,
        receive_count: AtomicUsize,
        connected: AtomicBool,
        sender: ::smol::channel::Sender<TestMessage>,
        state: AtomicCell<StateTable>,
    }
    impl Alice {
        fn new() -> (smol::channel::Receiver<TestMessage>, Self) {
            let (sender, receiver) = ::smol::channel::unbounded();
            let state = AtomicCell::new(StateTable::Init);
            (
                receiver,
                Self {
                    uuid: AtomicCell::new(Uuid::default()),
                    receive_count: AtomicUsize::default(),
                    connected: AtomicBool::default(),
                    sender,
                    state,
                },
            )
        }
    }
    impl Machine<TestMessage> for Alice {
        fn connected(&self, uuid: Uuid) {
            self.connected.store(true, Ordering::SeqCst);
            self.uuid.store(uuid);
        }
        fn disconnected(&self) { self.connected.store(false, Ordering::SeqCst); }
        fn receive(&self, cmd: TestMessage, sender: &mut MachineSender) {
            self.receive_count.fetch_add(1, Ordering::SeqCst);
            sender.send(self.sender.clone(), cmd);
        }
    }

    impl Machine<StateTable> for Alice {
        fn receive(&self, cmd: StateTable, _sender: &mut MachineSender) {
            let state = self.state.load();
            match cmd {
                StateTable::Init if state == StateTable::Init => {
                    println!("That was pointless, try starting me")
                },
                StateTable::Init => {
                    println!("I've already been started, so don't try to init me again")
                },
                StateTable::Start if state == StateTable::Start => {
                    println!("I'm already started, so that was pointless")
                },
                StateTable::Start => {
                    println!("I'm Alice, thanks for starting me");
                    self.state.store(StateTable::Start)
                },
                StateTable::Stop if state == StateTable::Init => {
                    println!("How rude, to be stopped before I get started");
                    self.state.store(StateTable::Stop)
                },
                StateTable::Stop if state == StateTable::Stop => {
                    println!("I'm already stopped, so that was pointless")
                },
                StateTable::Stop => {
                    println!("Alice has left the buiding");
                    self.state.store(StateTable::Stop)
                },
            }
        }
    }

    #[test]
    fn alice_receive_test_message() {
        let (receiver, alice) = Alice::new();
        let (machine, sender) = create::<TestMessage, _>(alice);
        assert_eq!(0, machine.receive_count.load(Ordering::SeqCst));

        for _ in 1 ..= 3 {
            sender.try_send(TestMessage::Test).ok();
        }
        thread::sleep(std::time::Duration::from_millis(20));
        assert_eq!(3, machine.receive_count.load(Ordering::SeqCst));
        let mut receive_count = 0;
        for _ in 1 ..= 3 {
            if let Ok(_cmd) = receiver.try_recv() {
                receive_count += 1;
            }
        }
        assert_eq!(3, receive_count);
    }

    #[test]
    fn alice_test_message_notifications() {
        let (_receiver, alice) = Alice::new();
        let (machine, sender) = create::<TestMessage, _>(alice);
        thread::sleep(std::time::Duration::from_millis(20));
        assert_eq!(true, machine.connected.load(Ordering::SeqCst));

        println!("machine={:#?}", machine);

        sender.close();
        thread::sleep(std::time::Duration::from_millis(20));
        assert_eq!(false, machine.connected.load(Ordering::SeqCst));
    }

    #[test]
    fn alice_test_message_and_state_table() {
        let (_receiver, alice) = Alice::new();
        // Get a sender to Alice that accepts TestMessage instructions
        let (alice, sender) = create::<TestMessage, _>(alice);
        // Extend alice and get a sender that accepts StateTable instructions
        let state_sender = extend::<StateTable, _>(&alice);
        // we can sent TestMessage to Alice
        sender.try_send(TestMessage::Test).ok();
        // and StateTable instructions too
        state_sender.try_send(StateTable::Init).ok();
        assert_eq!(StateTable::Init, alice.state.load());

        state_sender.try_send(StateTable::Start).ok();
        thread::sleep(std::time::Duration::from_millis(20));
        assert_eq!(StateTable::Start, alice.state.load());

        state_sender.try_send(StateTable::Stop).ok();
        thread::sleep(std::time::Duration::from_millis(20));
        assert_eq!(StateTable::Stop, alice.state.load());
    }
}
