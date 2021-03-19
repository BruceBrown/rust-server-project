use super::*;
use std::{sync::atomic::AtomicUsize, sync::Arc, time::Duration};

pub trait TestDriver {
    fn setup(&mut self);
    fn teardown(driver: Self);
    fn run(&self);
}

pub fn wait_for_notification(receiver: &TestMessageReceiver, _messages: usize, _duration: Duration) -> Result<(), ()> {
    let start = std::time::SystemTime::now();
    let executor = EXECUTOR.0[0].clone();
    let r = receiver.clone();
    smol::future::block_on(executor.run(async move {
        if let Ok(_cmd) = r.recv().await {
            let elapsed = start.elapsed().unwrap();
            println!("completed in {:#?}", elapsed);
        } else {
            println!("receiver got error");
        }
    }));
    Ok(())
}

/// DaisyChain will setup a linear network of machines in which a messages
/// received by a machine is forwarded to the next. Every machine in the
/// network will be visited by the message. Send 400 message in a network
/// of 4000 machines produces 1,600,000 message propagations. Additionally,
/// the propagation through the network is essentially a single pulse wave.
/// In this case, a pulse of 400 messages.
#[derive(Debug, SmartDefault)]
pub struct DaisyChainDriver {
    #[default = true]
    pub executor_per_thread: bool,

    #[default = 4]
    pub thread_count: usize,

    #[default = 4000]
    pub machine_count: usize,

    #[default = 200]
    pub message_count: usize,

    #[default = true]
    pub bound_queue: bool,

    #[default = 1]
    pub forwarding_multiplier: usize,

    #[default(Duration::from_secs(10))]
    pub duration: Duration,

    #[default(Vec::with_capacity(4010))]
    pub senders: Vec<TestMessageSender>,
    pub forwarders: Vec<Arc<Forwarder>>,

    pub first_sender: Option<TestMessageSender>,
    pub receiver: Option<TestMessageReceiver>,
    pub baseline: usize,
    pub exepected_message_count: usize,

    #[default(AtomicUsize::new(1))]
    pub iteration: AtomicUsize,
}
impl TestDriver for DaisyChainDriver {
    // setup the machines
    fn setup(&mut self) {
        smol::block_on(async {
            let (f, s) = if self.bound_queue {
                connect(Forwarder::new(1))
            } else {
                connect_unbounded(Forwarder::new(1))
            };
            self.forwarders.push(f);
            self.first_sender = Some(s.clone());
            let mut last_sender = s.clone();
            self.senders.push(s);
            for idx in 2 ..= self.machine_count {
                let (f, s) = if self.bound_queue {
                    connect(Forwarder::new(idx))
                } else {
                    connect_unbounded(Forwarder::new(idx))
                };
                self.forwarders.push(f);
                last_sender.send(TestMessage::AddSender(s.clone())).await.ok();
                last_sender
                    .send(TestMessage::ForwardingMultiplier(self.forwarding_multiplier))
                    .await
                    .ok();
                last_sender = s.clone();
                self.senders.push(s);
            }
            self.exepected_message_count = self.message_count * (self.forwarding_multiplier.pow((self.machine_count - 1) as u32));
            if self.forwarding_multiplier > 1 {
                log::info!("daisy_chain: expecting {} messages", self.exepected_message_count);
            }
            // turn the last into a notifier
            let (sender, receiver) = smol::channel::unbounded::<TestMessage>();
            last_sender
                .send(TestMessage::Notify(sender, self.exepected_message_count))
                .await
                .ok();
            self.receiver = Some(receiver);
            log::info!("daisy_chain: setup complete");
        })
    }

    // tear down the machines
    fn teardown(mut _daisy_chain: Self) {
        log::debug!("daisy_chain: tear-down started");
        log::info!("daisy_chain: tear-down complete");
    }

    // run a single iteration
    fn run(&self) {
        // let count = self.iteration.fetch_add(1, Ordering::SeqCst);
        // log::info!("daisy_chain iteration: {}", count);
        let first_sender = self.first_sender.clone();
        let message_count = self.message_count;
        EXECUTOR.0[0]
            .clone()
            .spawn(async move {
                if let Some(sender) = first_sender.as_ref() {
                    for msg_id in 0 .. message_count {
                        sender.send(TestMessage::TestData(msg_id)).await.ok();
                    }
                    println!("completed sending {} messages", message_count);
                }
            })
            .detach();
        if let Some(receiver) = self.receiver.as_ref() {
            println!("waiting for completion");
            if wait_for_notification(receiver, self.exepected_message_count, self.duration).is_err() {
                panic!("daisy_chain: completion notification failed");
            }
            println!("done");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_daisy_chain() {
        let mut config = DaisyChainDriver::default();
        config.machine_count = 100;
        config.message_count = 100;
        config.setup();
        assert_eq!(config.machine_count, config.forwarders.len());
        config.run();

        for f in &config.forwarders {
            assert_eq!(config.message_count, f.get_and_clear_received_count());
        }
        DaisyChainDriver::teardown(config);
    }

    #[test]
    fn large_daisy_chain() {
        default_channel_max.store(1000);
        let mut config = DaisyChainDriver::default();
        config.machine_count = 10_000;
        config.message_count = 20_000;

        config.setup();
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert_eq!(config.machine_count, config.forwarders.len());
        config.run();

        EXECUTOR.2.close();
        for f in &config.forwarders {
            let count = f.get_and_clear_received_count();
            if count != config.message_count {
                println!(
                    "fwd={} receive_count={} should have been expected_count={}",
                    f.get_id(),
                    count,
                    config.message_count
                );
            }
            assert_eq!(config.message_count, count);
        }
        DaisyChainDriver::teardown(config);
    }
}
