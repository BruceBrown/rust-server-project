#![allow(dead_code)]
use super::*;

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
    pub fn new(machine: Arc<dyn Machine<T>>, executor: Arc<::smol::Executor<'static>>, receiver: ::smol::channel::Receiver<T>) -> Self {
        let id = Uuid::new_v4();
        Self {
            id,
            machine,
            executor,
            receiver,
        }
    }

    // Start a Machine running. Once started, it runs until its receiver is closed.
    pub fn start(self) -> Arc<MachineAdapter<T>> {
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
