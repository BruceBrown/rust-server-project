use super::*;

#[derive(Debug, Clone)]
pub enum TestMessage {
    // Test is a unit-like instruction with no parameters
    Test,
    // TestData has a single parameter, as a tuple
    TestData(usize),
    // TestStruct is an example of passing a structure
    TestStruct(TestStruct),
    // TestCallback illustrates passing a sender and a structure to be sent back to the sender
    TestCallback(TestMessageSender, TestStruct),
    // AddSender can be implemented to push a sender onto a list of senders
    AddSender(TestMessageSender),
    // AddSenders can be implemented to push a vec of senders onto a list of senders
    AddSenders(Vec<TestMessageSender>),
    // RemoveAllSeners can be implemented to clear list of senders
    RemoveAllSenders,
    // Notify, is setup for a notification via TestData, where usize is a message count
    Notify(TestMessageSender, usize),
    // ForwardingMultiplier provides a parameter to the forwarder
    ForwardingMultiplier(usize),
    // Random message sending, illustrates that a variant struct can be used as well as a tuple
    ChaosMonkey {
        // A counter which is either incremented or decremented
        counter: u32,
        // The maximum value of the counter
        max: u32,
        // the type of mutation applied to the counter
        mutation: ChaosMonkeyMutation,
    },
}
// Generate these from MachineImpl
pub type TestMessageSender = smol::channel::Sender<TestMessage>;
pub type TestMessageReceiver = smol::channel::Receiver<TestMessage>;

impl MachineImpl for TestMessage {
    type Adapter = MachineBuilderTestMessage;
    type InstructionSet = TestMessage;
}

pub struct MachineAdapterTestMessage {}

#[derive(Debug, Clone)]
pub struct MachineSenderTestMessage {
    sender: smol::channel::Sender<TestMessage>,
    executor: std::sync::Arc<smol::Executor<'static>>,
}

impl TestMessage {
    pub fn advance(self) -> Self {
        match self {
            Self::ChaosMonkey { counter, max, mutation } => Self::advance_chaos_monkey(counter, max, mutation),
            _ => self,
        }
    }
    // return true if advancing will mutate, false if advance has no effect
    pub fn can_advance(&self) -> bool {
        match self {
            Self::ChaosMonkey { counter, mutation, .. } => *counter != 0 || mutation != &ChaosMonkeyMutation::Decrement,
            _ => false,
        }
    }
    // Advance the chaos monkey variant by increment the counter until it reaches its maximum value, then decrement it.
    // Once the counter reaches 0, no further advancement is performed.
    const fn advance_chaos_monkey(counter: u32, max: u32, mutation: ChaosMonkeyMutation) -> Self {
        match counter {
            0 => match mutation {
                ChaosMonkeyMutation::Increment => Self::ChaosMonkey {
                    counter: counter + 1,
                    max,
                    mutation,
                },
                ChaosMonkeyMutation::Decrement => Self::ChaosMonkey { counter, max, mutation },
            },
            c if c >= max => match mutation {
                ChaosMonkeyMutation::Increment => Self::ChaosMonkey {
                    counter,
                    max,
                    mutation: ChaosMonkeyMutation::Decrement,
                },
                ChaosMonkeyMutation::Decrement => Self::ChaosMonkey {
                    counter: counter - 1,
                    max,
                    mutation,
                },
            },
            _ => match mutation {
                ChaosMonkeyMutation::Increment => Self::ChaosMonkey {
                    counter: counter + 1,
                    max,
                    mutation,
                },
                ChaosMonkeyMutation::Decrement => Self::ChaosMonkey {
                    counter: counter - 1,
                    max,
                    mutation,
                },
            },
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ChaosMonkeyMutation {
    Increment,
    Decrement,
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub struct TestStruct {
    pub from_id: usize,
    pub received_by: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_chaos_monkey_variant() {
        let v = TestMessage::ChaosMonkey {
            counter: 0,
            max: 1,
            mutation: ChaosMonkeyMutation::Increment,
        };
        assert_eq!(true, v.can_advance());
        if let TestMessage::ChaosMonkey { counter, max, mutation } = v {
            assert_eq!(counter, 0);
            assert_eq!(max, 1);
            assert_eq!(mutation, ChaosMonkeyMutation::Increment);
        } else {
            assert_eq!(true, false)
        }
    }
    #[test]
    fn test_advance() {
        let v = TestMessage::ChaosMonkey {
            counter: 0,
            max: 1,
            mutation: ChaosMonkeyMutation::Increment,
        };
        let v = v.advance();
        if let TestMessage::ChaosMonkey { counter, max, mutation } = v {
            assert_eq!(counter, 1);
            assert_eq!(max, 1);
            assert_eq!(mutation, ChaosMonkeyMutation::Increment);
        } else {
            assert_eq!(true, false)
        }
        assert_eq!(true, v.can_advance());
    }
    #[test]
    fn test_advance_ends() {
        let v = TestMessage::ChaosMonkey {
            counter: 0,
            max: 1,
            mutation: ChaosMonkeyMutation::Increment,
        };
        let v = v.advance();
        let v = v.advance();
        assert_eq!(true, v.can_advance());
        let v = v.advance();
        if let TestMessage::ChaosMonkey { counter, max, mutation } = v {
            assert_eq!(counter, 0);
            assert_eq!(max, 1);
            assert_eq!(mutation, ChaosMonkeyMutation::Decrement);
        } else {
            assert_eq!(true, false)
        }
        assert_eq!(false, v.can_advance());
        let v = v.advance();
        if let TestMessage::ChaosMonkey { counter, max, mutation } = v {
            assert_eq!(counter, 0);
            assert_eq!(max, 1);
            assert_eq!(mutation, ChaosMonkeyMutation::Decrement);
        } else {
            assert_eq!(true, false)
        }
    }
}
