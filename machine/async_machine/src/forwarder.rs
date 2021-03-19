use super::*;
use parking_lot::Mutex;

#[derive(Default, Debug)]
pub struct Forwarder {
    /// a id, mosly used for logging
    id: usize,
    /// The mutable data...
    data: Mutex<ForwarderMutable>,
}

/// This is the mutable part of the Forwarder
#[derive(Debug, SmartDefault)]
pub struct ForwarderMutable {
    /// a id, mosly used for logging
    id: usize,
    /// collection of senders, each will be sent any received message.
    senders: Vec<TestMessageSender>,
    /// received_count is the count of messages received by this forwarder.
    received_count: usize,
    /// send_count is the count of messages sent by this forwarder.
    send_count: usize,
    /// notify_count is compared against received_count for means of notifcation.
    notify_count: usize,
    /// notify_sender is sent a TestData message with the data being the number of messages received.
    notify_sender: Option<TestMessageSender>,
    /// forwarding multiplier
    #[default = 1]
    forwarding_multiplier: usize,
    // Chaos monkey random
    #[default(Uniform::from(0..1))]
    range: Uniform<usize>,
    // for TestData, this is the next in sequence
    next_seq: usize,
}
impl ForwarderMutable {
    /// get an index suitable for obtaining a random sender from the senders vector
    fn get_monkey_fwd(&mut self) -> usize {
        let mut rng = thread_rng();
        self.range.sample(&mut rng)
    }
    fn drop_all_senders(&mut self) {
        self.senders.clear();
        self.notify_sender = None;
    }

    /// if msg is TestData, validate the sequence or reset if 0
    fn validate_sequence(&mut self, msg: TestMessage) -> Result<TestMessage, TestMessage> {
        match msg {
            TestMessage::TestData(seq) if seq == self.next_seq => self.next_seq += 1,
            TestMessage::TestData(seq) if seq == 0 => self.next_seq = 1,
            TestMessage::TestData(_) => return Err(msg),
            _ => (),
        }
        // bump received count
        self.received_count += 1;
        Ok(msg)
    }

    /// If msg is a configuration msg, handle it otherwise return it as an error
    fn handle_config(&mut self, msg: TestMessage) -> Result<(), TestMessage> {
        match msg {
            TestMessage::Notify(sender, on_receive_count) => {
                self.notify_sender = Some(sender);
                self.notify_count = on_receive_count;
            },
            TestMessage::AddSender(sender) => {
                self.senders.push(sender);
                self.range = Uniform::from(0 .. self.senders.len());
            },
            TestMessage::AddSenders(senders) => {
                self.senders = senders;
                self.range = Uniform::from(0 .. self.senders.len());
            },
            TestMessage::ForwardingMultiplier(count) => self.forwarding_multiplier = count,
            TestMessage::RemoveAllSenders => self.drop_all_senders(),
            msg => return Err(msg),
        }
        Ok(())
    }

    /// handle the action messages
    fn handle_action(&mut self, message: TestMessage, id: usize, sender: &mut dyn MachineSender<TestMessage>) {
        match message {
            TestMessage::ChaosMonkey { .. } if message.can_advance() => {
                let idx = self.get_monkey_fwd();
                sender.send(self.senders[idx].clone(), message.advance());
            },
            TestMessage::ChaosMonkey { .. } => {
                if let Some(notifier) = self.notify_sender.as_ref() {
                    sender.send(notifier.clone(), TestMessage::TestData(0));
                }
            },
            TestMessage::TestData(_) => {
                for s in &self.senders {
                    for _ in 0 .. self.forwarding_multiplier {
                        // println!("fwd={} sending=TestData({}) to={}", self.id, self.send_count, sender.get_id());
                        sender.send(s.clone(), TestMessage::TestData(self.send_count));
                        // println!("fwd={} sent=TestData({}) to={}", self.id, self.send_count, sender.get_id());
                        self.send_count += 1;
                    }
                }
            },
            TestMessage::TestCallback(_s, mut test_struct) => {
                test_struct.received_by = id;
                // sender.send(&s, TestMessage::TestStruct(test_struct));
            },
            _ => self.senders.iter().for_each(|s| {
                for _ in 0 .. self.forwarding_multiplier {
                    sender.send(s.clone(), message.clone());
                }
            }),
        }
    }

    /// handle sending out a notification and resetting counters when notificaiton is sent
    fn handle_notification(&mut self, sender: &mut dyn MachineSender<TestMessage>) {
        if self.received_count == self.notify_count {
            // log::trace!("received {} out of {}", count, mutable.notify_count);
            if let Some(notifier) = &self.notify_sender {
                sender.send(notifier.clone(), TestMessage::TestData(self.received_count));
            }
        }
    }

    /// get the current received count and clear counters
    fn get_and_clear_received_count(&mut self) -> usize {
        let received_count = self.received_count;
        self.received_count = 0;
        self.send_count = 0;
        received_count
    }

    pub const fn get_id(&self) -> usize { self.id }
}

impl Forwarder {
    pub fn new(id: usize) -> Self {
        let res = Self { id, ..Default::default() };
        {
            let mut data = res.data.lock();
            data.id = id;
        }
        res
    }
    pub const fn get_id(&self) -> usize { self.id }

    pub fn get_and_clear_received_count(&self) -> usize { self.data.lock().get_and_clear_received_count() }
}

impl Machine<TestMessage> for Forwarder {
    fn disconnected(&self) { self.data.lock().drop_all_senders(); }

    fn receive(&self, message: TestMessage, sender: &mut dyn MachineSender<TestMessage>) {
        if let Some(mut data) = self.data.try_lock() {
            match data.handle_config(message) {
                Ok(_) => (),
                Err(msg) => match data.validate_sequence(msg) {
                    Ok(msg) => {
                        data.handle_action(msg, self.id, sender);
                        data.handle_notification(sender);
                    },
                    Err(msg) => panic!("sequence error fwd {}, msg {:#?}", self.id, msg),
                },
            }
        } else {
            panic!("failed to acquire mutable lock for forwarder");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_forwarder() {
        let (machine, sender) = connect(Forwarder::new(1));
        assert_eq!(0, machine.data.lock().get_and_clear_received_count());
        println!("machine={:#?}", machine);
        sender.try_send(TestMessage::Test).ok();
        std::thread::sleep(std::time::Duration::from_millis(20));
        assert_eq!(1, machine.data.lock().get_and_clear_received_count());
    }
}
