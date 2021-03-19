use super::*;
use rand::distributions::{Distribution, Uniform};

/// ChaosMonkey will setup a network of machines in which a message received by any machine
/// can be forwarded to any other machine, including itself. To break the cyclic nature of this,
/// the message has a counter, inflection, and mutation (increment or decrement). The counter
/// starts with a value of 0 and a mutation of increment, as such each time it is forwarded,
/// the counter is incremented until it reaches the infection, at which point the mutation
/// is changed to decrement. The counter will decrement until it reaches 0, at which point
/// it will cease to be forwarded and a notification will be sent indicating that the
/// message forwarding for that message is complete. When all messages reach 0, the test
/// is complete.
///
/// For example, if the inflection is 3, the message with fwd 0, 1, 2, 3, 3, 2, 1, 0.
/// If there are 400 messages, and an inflection of 9 20x400 messages will be propagated.
/// This is purely random, if you have 4000 machines, in this scenerio each machine would be
/// visted by ~2 messages.
///
/// The message count represents concurrent number of messages flowing through the machines,
/// while the inflection value represents the lifetime of the message. Varing the machine count
/// varies the number of messages a machine may receive.
#[derive(Debug, SmartDefault)]
pub struct ChaosMonkeyDriver {
    #[default = 3000]
    pub machine_count: usize,

    #[default = 200]
    pub message_count: usize,

    #[default = 99]
    pub inflection_value: u32,

    #[default = true]
    pub bound_queue: bool,

    #[default(Duration::from_secs(10))]
    pub duration: Duration,

    #[default(Vec::with_capacity(3010))]
    pub senders: Vec<TestMessageSender>,

    pub receiver: Option<TestMessageReceiver>,
    pub baseline: usize,
    pub forwarders: Vec<Arc<Forwarder>>,
}

impl TestDriver for ChaosMonkeyDriver {
    // setup the machines
    fn setup(&mut self) { smol::block_on(self.async_setup()); }

    // tear down the machines
    fn teardown(mut _chaos_monkey: Self) {
        log::debug!("chaos_monkey: tear-down started");
        log::info!("chaos_monkey: tear-down complete");
    }

    // run a single iteration
    fn run(&self) {
        let range = Uniform::from(0 .. self.senders.len());
        let mut rng = rand::rngs::OsRng::default();
        let senders = self.senders.clone();
        let message_count = self.message_count;
        let inflection_value = self.inflection_value;
        get_executor()
            .clone()
            .spawn(async move {
                for _ in 0 .. message_count {
                    let idx = range.sample(&mut rng);
                    senders[idx]
                        .send(TestMessage::ChaosMonkey {
                            counter: 0,
                            max: inflection_value,
                            mutation: ChaosMonkeyMutation::Increment,
                        })
                        .await
                        .ok();
                }
            })
            .detach();
        if let Some(receiver) = self.receiver.as_ref() {
            println!("waiting for completion");
            if wait_for_notification(receiver, self.message_count, self.duration).is_err() {
                panic!("chaos_monkey: completion notification failed");
            }
            println!("done");
        }
    }
}

impl ChaosMonkeyDriver {
    async fn async_setup(&mut self) {
        let mut forwarders: Vec<Arc<Forwarder>> = Vec::new();
        smol::block_on(async {
            // we're going to create N machines, each having N senders, plus a notifier.
            for idx in 1 ..= self.machine_count {
                let (f, s) = if self.bound_queue {
                    machine::create(Forwarder::new(idx))
                } else {
                    machine::create_unbounded(Forwarder::new(idx))
                };
                self.senders.push(s);
                forwarders.push(f);
            }
            let (f, notifier) = if self.bound_queue {
                machine::create(Forwarder::new(self.machine_count + 1))
            } else {
                machine::create_unbounded(Forwarder::new(self.machine_count + 1))
            };
            forwarders.push(f);
            log::debug!("chaos_monkey: monkeys created");
            // form a complete map by sending all the monkey's senders to each monkey
            for s1 in &self.senders {
                let cloned_senders = self.senders.clone();
                s1.send(TestMessage::AddSenders(cloned_senders)).await.ok();
                // chaos monkey ignores the count
                s1.send(TestMessage::Notify(notifier.clone(), 0)).await.ok();
            }
            let (sender, receiver) = smol::channel::unbounded::<TestMessage>();
            notifier.send(TestMessage::Notify(sender, self.message_count)).await.ok();
            self.receiver = Some(receiver);
            log::info!("chaos_monkey: setup complete");
            self.forwarders = forwarders;
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_chaos_monkey() {
        // Each monkey may receive a message, he'll send it to another monkey until the
        // message has been sent 200 times. There will be 10 messages sent, for a total
        // of 2000 message, and 10 notifications, one from each monkey that received the
        // final message, for a total of 2010 messages.
        let mut config = ChaosMonkeyDriver::default();
        config.machine_count = 10;
        config.message_count = 10;
        let expected = (config.inflection_value as usize + 1) * 2 * config.message_count + config.message_count;
        config.setup();
        config.run();
        let mut total = 0;
        for f in &config.forwarders {
            let count = f.get_and_clear_received_count();
            total += count;
        }
        println!("total messages received={}, expected={}", total, expected);
        ChaosMonkeyDriver::teardown(config);
    }

    #[test]
    #[ignore]
    fn large_chaos_monkey() {
        set_default_channel_max(1000);

        // This time we're going to ramp it up and push 40_020_000 messages through.
        let mut config = ChaosMonkeyDriver::default();
        config.machine_count = 1_000;
        config.message_count = 20_000;
        config.inflection_value = 999;
        let expected = (config.inflection_value as usize + 1) * 2 * config.message_count + config.message_count;
        println!("expecting message count={}", expected);
        config.setup();
        config.run();
        let mut total = 0;
        for f in &config.forwarders {
            let count = f.get_and_clear_received_count();
            total += count;
        }
        println!("total messages received={}, expected={}", total, expected);
        ChaosMonkeyDriver::teardown(config);
    }
}
