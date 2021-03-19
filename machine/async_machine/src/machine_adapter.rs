use super::*;
use crossbeam::atomic::AtomicCell;
use once_cell::sync::Lazy;
use smol::{self, channel, future, Executor};
use std::{
    fmt,
    panic::catch_unwind,
    sync::atomic::{AtomicUsize, Ordering},
    sync::Arc,
    thread,
};
use uuid::Uuid;

type SharedMachineAdapter = Arc<MachineAdapter>;
type SharedMachine<T> = Arc<T>;

// This is all of the glue that holds everything together. The API is pretty brain dead simple, it consists
// of a call to connect(), which returns a wrapped object and sender for that object. Its fine to drop the
// wrapped object. Dropping the sender will clean everything up associated with the connected object.
// the sender has a single method, send(). It will block until the send is accepted into the wrapped objects
// channel, or the channel is closed.
//

static EXECUTOR_SEED: AtomicUsize = AtomicUsize::new(0);

pub static EXECUTOR: Lazy<(Vec<Arc<Executor<'_>>>, Vec<thread::JoinHandle<()>>, smol::channel::Sender<()>)> = Lazy::new(|| {
    let handles: Vec<thread::JoinHandle<()>> = Vec::new();
    let (s, r) = smol::channel::unbounded::<()>();
    let mut send_executors: Vec<Arc<Executor<'_>>> = Vec::new();
    let num_threads = 4;
    for n in 1 ..= num_threads {
        let e = Arc::new(Executor::new());
        let r = r.clone();
        send_executors.push(e.clone());
        thread::Builder::new()
            .name(format!("send-ex-{}", n))
            .spawn(move || loop {
                catch_unwind(|| future::block_on(e.run(async { r.recv().await }))).ok();
            })
            .expect("cannot spawn executor thread");
    }
    (send_executors.clone(), handles, s)
});

#[allow(non_upper_case_globals)]
pub static default_channel_max: AtomicCell<usize> = AtomicCell::new(20);

#[doc(hidden)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, SmartDefault)]
#[allow(dead_code)]
pub enum MachineState {
    #[default]
    New,
    Waiting,
    Ready,
    Running,
    SendBlock,
    RecvBlock,
    // Disconnected,
    Dead,
}

// A thread-safe wrapped state, which can be cloned.
#[doc(hidden)]
pub type SharedMachineState = Arc<AtomicCell<MachineState>>;

pub trait MachineImpl: 'static + Send + Sync {
    type Adapter;
    type InstructionSet: Send + Sync;
}

// All machines must implement a Machine<T> for each instruction set they support.
pub trait Machine<T>: Send + Sync
where
    T: 'static + Send + Sync,
{
    fn receive(&self, cmd: T, sender: &mut dyn MachineSender<T>);
    fn disconnected(&self) {}
    fn connected(&self, _uuid: uuid::Uuid) {}
}

pub trait MachineSender<T>: Send + Sync
where
    T: 'static + Send + Sync,
{
    // type InstructionSet: MachineImpl;

    fn send(&mut self, channel: channel::Sender<T>, cmd: T);
}
impl<T> std::fmt::Debug for dyn MachineSender<T>
where
    T: Send + Sync,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result { write!(f, "MachineSender {{ .. }}") }
}

pub fn connect<T, P>(
    machine: T,
) -> (
    SharedMachine<T>,
    channel::Sender<<<P as MachineImpl>::Adapter as MachineBuilder>::InstructionSet>,
)
where
    T: 'static + Machine<P> + Machine<<<P as MachineImpl>::Adapter as MachineBuilder>::InstructionSet>,
    P: MachineImpl,
    <P as MachineImpl>::Adapter: MachineBuilder,
{
    let channel_max = default_channel_max.load();
    let (machine, sender, _adapter) = <<P as MachineImpl>::Adapter as MachineBuilder>::bounded(machine, channel_max);
    (machine, sender)
}

pub fn connect_unbounded<T, P>(
    machine: T,
) -> (
    SharedMachine<T>,
    channel::Sender<<<P as MachineImpl>::Adapter as MachineBuilder>::InstructionSet>,
)
where
    T: 'static + Machine<P> + Machine<<<P as MachineImpl>::Adapter as MachineBuilder>::InstructionSet>,
    P: MachineImpl,
    <P as MachineImpl>::Adapter: MachineBuilder,
{
    let (machine, sender, _adapter) = <<P as MachineImpl>::Adapter as MachineBuilder>::unbounded(machine);
    (machine, sender)
}

pub trait MachineBuilder {
    type InstructionSet: MachineImpl;

    fn bounded<T>(machine: T, capacity: usize) -> (SharedMachine<T>, channel::Sender<Self::InstructionSet>, SharedMachineAdapter)
    where
        T: 'static + Machine<Self::InstructionSet>,
        <Self as MachineBuilder>::InstructionSet: Send,
    {
        let channel = channel::bounded::<Self::InstructionSet>(capacity);
        Self::common_create(machine, channel)
    }

    fn unbounded<T>(machine: T) -> (SharedMachine<T>, channel::Sender<Self::InstructionSet>, SharedMachineAdapter)
    where
        T: 'static + Machine<Self::InstructionSet>,
        <Self as MachineBuilder>::InstructionSet: Send,
    {
        let channel = channel::unbounded::<Self::InstructionSet>();
        Self::common_create(machine, channel)
    }

    fn common_create<T>(
        machine: T, channel: (channel::Sender<Self::InstructionSet>, channel::Receiver<Self::InstructionSet>),
    ) -> (SharedMachine<T>, channel::Sender<Self::InstructionSet>, SharedMachineAdapter)
    where
        T: 'static + Machine<Self::InstructionSet>,
        <Self as MachineBuilder>::InstructionSet: Send,
    {
        let machine: SharedMachine<T> = Arc::new(machine);
        let (sender, adapter) = Self::common_addition(&machine, channel);
        (machine, sender, adapter)
    }

    fn common_addition<T>(
        machine: &SharedMachine<T>, channel: (channel::Sender<Self::InstructionSet>, channel::Receiver<Self::InstructionSet>),
    ) -> (channel::Sender<Self::InstructionSet>, SharedMachineAdapter)
    where
        T: 'static + Machine<Self::InstructionSet>,
    {
        let machine = Arc::clone(machine) as Arc<dyn Machine<Self::InstructionSet>>;
        Self::make_adapter(machine, channel)
    }

    fn make_adapter(
        machine: Arc<dyn Machine<Self::InstructionSet>>,
        channel: (channel::Sender<Self::InstructionSet>, channel::Receiver<Self::InstructionSet>),
    ) -> (channel::Sender<Self::InstructionSet>, SharedMachineAdapter);
}

pub struct MachineBuilderTestMessage {}

impl MachineBuilder for MachineBuilderTestMessage {
    type InstructionSet = TestMessage;
    fn make_adapter(
        machine: Arc<dyn Machine<Self::InstructionSet>>,
        channel: (channel::Sender<Self::InstructionSet>, channel::Receiver<Self::InstructionSet>),
    ) -> (channel::Sender<Self::InstructionSet>, SharedMachineAdapter) {
        let (s, r) = channel;
        let next = EXECUTOR_SEED.fetch_add(1, Ordering::SeqCst);
        let idx = next % EXECUTOR.0.len();
        let executor = EXECUTOR.0[idx].clone();
        let adapter = MachineAdapter::new(machine, executor, r);
        let adapter = adapter.start();
        (s, adapter)
    }
}

pub struct TestMessageMachine {}
impl Machine<TestMessage> for TestMessageMachine {
    fn receive(&self, _cmd: TestMessage, _sender: &mut dyn MachineSender<TestMessage>) {}
}

#[derive(SmartDefault)]
pub struct MachineAdapter {
    #[default(Uuid::new_v4())]
    // The id is assigned on creation, and is intended for to be used in logging
    id: Uuid,

    // The key is assigned when the machine is assigned to the collective. When a
    // machine is removed from the collective, it's key can be re-issued.
    pub key: AtomicUsize,

    // The state of the machine. Its an Arc<AtomicCell<MachineState>> allowing
    // it to be shared with other adapters
    pub state: SharedMachineState,

    pub task_id: AtomicUsize,

    #[default(Arc::new(TestMessageMachine{}))]
    pub machine: Arc<dyn Machine<TestMessage>>,
    pub executor: Arc<smol::Executor<'static>>,

    #[default(smol::channel::unbounded::<TestMessage>().1)]
    pub receiver: smol::channel::Receiver<TestMessage>,
}

type FutureQueue = Vec<(channel::Sender<TestMessage>, TestMessage)>;

#[derive(Default)]
pub struct SenderAdapter {
    queue: FutureQueue,
}

impl fmt::Debug for SenderAdapter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "#ChannelSender {{ .. }}") }
}

impl MachineSender<TestMessage> for SenderAdapter {
    fn send(&mut self, channel: channel::Sender<TestMessage>, cmd: TestMessage) { self.queue.push((channel, cmd)); }
}

impl std::fmt::Debug for MachineAdapter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "#MachineAdapter {{ .. }}") }
}

impl MachineAdapter {
    #[inline]
    pub fn new(
        machine: Arc<dyn Machine<TestMessage>>, executor: Arc<smol::Executor<'static>>, receiver: smol::channel::Receiver<TestMessage>,
    ) -> Self {
        Self {
            machine,
            executor,
            receiver,
            ..Self::default()
        }
    }

    pub fn start(self) -> Arc<MachineAdapter> {
        let r = self.receiver.clone();
        let machine = self.machine.clone();
        let id = self.id;

        let adapter = Arc::new(self);
        adapter
            .executor
            .spawn(async move {
                let mut sender = SenderAdapter::default();
                machine.connected(id);
                while let Ok(cmd) = r.recv().await {
                    machine.receive(cmd, &mut sender);
                    for (s, cmd) in sender.queue.drain(..) {
                        s.send(cmd).await.ok();
                    }
                }
                machine.disconnected();
            })
            .detach();
        adapter
    }
    #[inline]
    pub const fn get_id(&self) -> Uuid { self.id }
    #[inline]
    pub fn get_key(&self) -> usize { self.key.load(Ordering::SeqCst) }
    #[inline]
    pub fn get_state(&self) -> MachineState { self.state.load() }
    #[inline]
    pub fn is_dead(&self) -> bool { self.state.load() == MachineState::Dead }
    #[inline]
    pub fn is_running(&self) -> bool { self.state.load() == MachineState::Running }
    #[inline]
    pub fn is_send_blocked(&self) -> bool { self.state.load() == MachineState::SendBlock }

    #[inline]
    pub fn compare_and_exchange_state(&self, current: MachineState, new: MachineState) -> Result<MachineState, MachineState> {
        self.state.compare_exchange(current, new)
    }
    #[inline]
    pub fn clear_task_id(&self) { self.task_id.store(0, Ordering::SeqCst); }
    #[inline]
    pub fn get_task_id(&self) -> usize { self.task_id.load(Ordering::SeqCst) }
    #[inline]
    pub fn set_task_id(&self, id: usize) { self.task_id.store(id, Ordering::SeqCst); }
    #[inline]
    pub fn set_state(&self, new: MachineState) { self.state.store(new); }
    #[inline]
    pub fn clone_state(&self) -> SharedMachineState { self.state.clone() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    // A trivial machine
    #[derive(Debug, Default)]
    struct Alice {
        uuid: AtomicCell<Uuid>,
        receive_count: AtomicUsize,
        connected: AtomicBool,
    }
    impl Machine<TestMessage> for Alice {
        fn connected(&self, uuid: Uuid) {
            self.connected.store(true, Ordering::SeqCst);
            self.uuid.store(uuid);
        }
        fn disconnected(&self) { self.connected.store(false, Ordering::SeqCst); }
        fn receive(&self, _cmd: TestMessage, _sender: &mut dyn MachineSender<TestMessage>) {
            self.receive_count.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn machine_receive() {
        let (machine, sender) = connect(Alice::default());
        assert_eq!(0, machine.receive_count.load(Ordering::SeqCst));

        for _ in 1 ..= 3 {
            sender.try_send(TestMessage::Test).ok();
        }
        thread::sleep(std::time::Duration::from_millis(20));
        assert_eq!(3, machine.receive_count.load(Ordering::SeqCst));
    }

    #[test]
    fn machine_notifications() {
        let (machine, sender) = connect(Alice::default());
        thread::sleep(std::time::Duration::from_millis(20));
        assert_eq!(true, machine.connected.load(Ordering::SeqCst));

        println!("machine={:#?}", machine);

        sender.close();
        thread::sleep(std::time::Duration::from_millis(20));
        assert_eq!(false, machine.connected.load(Ordering::SeqCst));
    }
}
