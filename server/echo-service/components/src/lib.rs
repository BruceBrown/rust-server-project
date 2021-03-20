use machine_impl::*;
use smart_default::*;

use machine_foundation::{get_executor, BackgroundTask};

use std::sync::Arc;

use atomic_refcell::AtomicRefCell;
use smol::{channel, lock::Mutex};

mod net_instructionset;
mod network;
mod service;

pub use net_instructionset::{NetCmd, NetConnId, NetReceiver, NetSender};
pub use network::NetCore;
pub use service::{ServerService, ServiceError, ServiceResult, ServiceState, ServiceStateTransition};

#[cfg(test)]
mod tests {}
