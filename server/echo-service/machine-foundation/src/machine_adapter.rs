use super::*;
use crossbeam::atomic::AtomicCell;

/// The default bounded machine size.
#[allow(non_upper_case_globals)]
static default_channel_max: AtomicCell<usize> = AtomicCell::new(20);

/// Set the default number of threads to use, returning the previous value. If 0, the framework will default to the
/// number of CPUs available.
pub fn set_default_channel_max(capacity: usize) -> usize {
    let res = get_default_num_threads();
    default_channel_max.store(capacity);
    res
}

/// Get the default number of threads to use. If 0, the framework will default to the
/// number of CPUs available.
pub fn get_default_channel_max() -> usize { default_channel_max.load() }

#[cfg(test)]
mod tests {

    use super::machine::*;
    use super::*;
    use crossbeam::atomic::AtomicCell;
    use instruction_set::*;
    use std::{
        sync::atomic::{AtomicBool, AtomicUsize, Ordering},
        thread,
    };
    use uuid::Uuid;

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
    fn alice_create_bounded_default() {
        let (_, alice) = Alice::new();
        let (alice, sender) = create::<TestMessage, _>(alice);
        assert_eq!(Some(get_default_channel_max()), sender.capacity());

        let sender = extend::<StateTable, _>(&alice);
        assert_eq!(Some(get_default_channel_max()), sender.capacity());
    }

    #[test]
    fn alice_create_bounded() {
        let (_, alice) = Alice::new();
        let (alice, sender) = create_with_capacity::<TestMessage, _>(alice, 1000);
        assert_eq!(Some(1000), sender.capacity());

        let sender = extend_with_capacity::<StateTable, _>(&alice, 500);
        assert_eq!(Some(500), sender.capacity());
    }

    #[test]
    fn alice_create_unbounded() {
        let (_, alice) = Alice::new();
        let (alice, sender) = create_unbounded::<TestMessage, _>(alice);
        assert_eq!(None, sender.capacity());

        let sender = extend_unbounded::<StateTable, _>(&alice);
        assert_eq!(None, sender.capacity());
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
