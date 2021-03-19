use super::*;

/// All services must implement ServerService
pub trait ServerService {
    /// Get the name of the service.
    fn get_name(&self) -> &str;
    /// Get the count of things to drain.
    fn get_drain_count(&self) -> usize;
    /// Return true if drained
    fn is_drained(&self) -> bool { self.get_drain_count() == 0 }
    /// Start the service. Generally, this prepares the service for running.
    fn start(&mut self) -> Result<(), std::io::Error>;
    /// Run the service.
    fn run(&mut self) -> Result<(), std::io::Error>;
    /// Stop the service from accepting new request or connections, continue processing outstanding requests or connections.
    fn drain(&mut self) -> Result<(), std::io::Error>;
    /// Stop the service, closing any requests or connections.
    fn stop(&mut self) -> Result<(), std::io::Error>;
}

/// ServiceStateTransiion provides notification of a ServiceState transition.
pub trait ServiceStateTransition {
    /// The will_start method is called before transitioning to ServiceState::Started. The state
    /// is the current state.
    fn will_start(&mut self, state: &ServiceState);
    /// The will_run method is called before transitioning to ServiceState::Running. The state
    /// is the current state.
    fn will_run(&mut self, state: &ServiceState);
    /// The will_drain method is called before transitioning to ServiceState::Draining. The state
    /// is the current state.
    fn will_drain(&mut self, state: &ServiceState);
    /// The will_stop method is called before transitioning to ServiceState::Stopped. The state
    /// is the current state.
    fn will_stop(&mut self, state: &ServiceState);
}

/// ServiceState is the state of the service.
#[derive(Debug, Copy, Clone, Eq, PartialEq, SmartDefault)]
pub enum ServiceState {
    #[default]
    Init,
    Started,
    Running,
    Draining,
    Stopped,
}

impl ServiceState {
    /// Attempt to transition to the Started state.
    pub fn start(&mut self) { self.start_with_notification(None) }

    /// Attempt to transition to the Started state, signalling state transition.
    pub fn start_with_notification(&mut self, on_transition: Option<&mut dyn ServiceStateTransition>) {
        if self.can_start() {
            if let Some(notifier) = on_transition {
                notifier.will_start(self);
            }
            *self = Self::Started
        }
    }

    /// Attempt to transition to the Running state.
    pub fn run(&mut self) { self.run_with_notification(None) }

    /// Attempt to transition to the Running state, signalling state transition.
    pub fn run_with_notification(&mut self, on_transition: Option<&mut dyn ServiceStateTransition>) {
        if self.can_run() {
            if let Some(notifier) = on_transition {
                notifier.will_run(self);
            }
            *self = Self::Running
        }
    }
    /// Attempt to transition to the Draining state.
    pub fn drain(&mut self) { self.drain_with_notification(None) }

    /// Attempt to transition to the Draining state, signalling state transition.
    pub fn drain_with_notification(&mut self, on_transition: Option<&mut dyn ServiceStateTransition>) {
        if self.can_drain() {
            if let Some(notifier) = on_transition {
                notifier.will_drain(self);
            }
            *self = Self::Draining
        }
    }
    /// Attempt to transition to the Stopped state.
    pub fn stop(&mut self) { self.stop_with_notification(None) }

    /// Attempt to transition to the Stopped state, signalling state transition.
    pub fn stop_with_notification(&mut self, on_transition: Option<&mut dyn ServiceStateTransition>) {
        if self.can_stop() {
            if let Some(notifier) = on_transition {
                notifier.will_stop(self);
            }
            *self = Self::Stopped
        }
    }

    /// Return true if state can transition to Started.
    pub fn can_start(self) -> bool { self == Self::Init }

    /// Return true if state can transition to Running.
    pub fn can_run(self) -> bool { self == Self::Started }

    /// Return true if state can transition to Draining.
    pub fn can_drain(self) -> bool { self == Self::Running }

    /// Return true if state can transition to Stopped.
    pub fn can_stop(self) -> bool { self != Self::Stopped }

    /// return trye if state is running
    pub fn is_running(self) -> bool { self == Self::Running }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Debug, Default)]
    struct Notifier {
        started: bool,
        running: bool,
        draining: bool,
        stopped: bool,
    }
    impl ServiceStateTransition for Notifier {
        fn will_start(&mut self, state: &ServiceState) {
            assert_eq!(true, state.can_start());
            self.started = true;
        }
        fn will_run(&mut self, state: &ServiceState) {
            assert_eq!(true, state.can_run());
            self.running = true;
        }
        fn will_drain(&mut self, state: &ServiceState) {
            assert_eq!(true, state.can_drain());
            self.draining = true;
        }
        fn will_stop(&mut self, state: &ServiceState) {
            assert_eq!(true, state.can_stop());
            self.stopped = true;
        }
    }

    #[test]
    fn service_state_advance_init_simpl() {
        let mut state = ServiceState::default();
        assert_eq!(state, ServiceState::Init);
        assert_eq!(false, state.is_running());

        state.start();
        assert_eq!(state, ServiceState::Started);
        assert_eq!(false, state.is_running());

        state.run();
        assert_eq!(state, ServiceState::Running);
        assert_eq!(true, state.is_running());

        state.drain();
        assert_eq!(state, ServiceState::Draining);
        assert_eq!(false, state.is_running());

        state.stop();
        assert_eq!(state, ServiceState::Stopped);
        assert_eq!(false, state.is_running());
    }

    #[test]
    fn service_state_advance_init() {
        let mut notifier = Notifier::default();
        let mut state = ServiceState::default();
        assert_eq!(state, ServiceState::Init);

        let test_state = ServiceState::Init;
        state = test_state;
        assert_eq!(true, state.can_start());
        assert_eq!(true, state.can_stop());
        assert_eq!(false, state.can_drain());
        assert_eq!(false, state.can_run());

        state = test_state;
        state.start_with_notification(Some(&mut notifier));
        assert_eq!(true, notifier.started);
        assert_eq!(state, ServiceState::Started);

        state = test_state;
        state.run_with_notification(Some(&mut notifier));
        assert_eq!(false, notifier.running);
        assert_eq!(state, test_state);

        state = test_state;
        state.drain_with_notification(Some(&mut notifier));
        assert_eq!(false, notifier.draining);
        assert_eq!(state, test_state);

        state = test_state;
        state.stop_with_notification(Some(&mut notifier));
        assert_eq!(true, notifier.stopped);
        assert_eq!(state, ServiceState::Stopped);
    }
    #[test]
    fn service_state_advance_started() {
        let mut notifier = Notifier::default();
        let test_state = ServiceState::Started;
        let mut state = test_state;
        assert_eq!(false, state.can_start());
        assert_eq!(true, state.can_stop());
        assert_eq!(false, state.can_drain());
        assert_eq!(true, state.can_run());

        state = test_state;
        state.start_with_notification(Some(&mut notifier));
        assert_eq!(false, notifier.started);
        assert_eq!(state, test_state);

        state = test_state;
        state.run_with_notification(Some(&mut notifier));
        assert_eq!(true, notifier.running);
        assert_eq!(state, ServiceState::Running);

        state = test_state;
        state.drain_with_notification(Some(&mut notifier));
        assert_eq!(false, notifier.draining);
        assert_eq!(state, test_state);

        state = test_state;
        state.stop_with_notification(Some(&mut notifier));
        assert_eq!(true, notifier.stopped);
        assert_eq!(state, ServiceState::Stopped);
    }

    #[test]
    fn service_state_advance_running() {
        let mut notifier = Notifier::default();
        let test_state = ServiceState::Running;
        let mut state = test_state;
        assert_eq!(false, state.can_start());
        assert_eq!(true, state.can_stop());
        assert_eq!(true, state.can_drain());
        assert_eq!(false, state.can_run());

        state = test_state;
        state.start_with_notification(Some(&mut notifier));
        assert_eq!(false, notifier.started);
        assert_eq!(state, test_state);

        state = test_state;
        state.run_with_notification(Some(&mut notifier));
        assert_eq!(false, notifier.running);
        assert_eq!(state, test_state);

        state = test_state;
        state.drain_with_notification(Some(&mut notifier));
        assert_eq!(true, notifier.draining);
        assert_eq!(state, ServiceState::Draining);

        state = test_state;
        state.stop_with_notification(Some(&mut notifier));
        assert_eq!(true, notifier.stopped);
        assert_eq!(state, ServiceState::Stopped);
    }

    #[test]
    fn service_state_advance_draining() {
        let mut notifier = Notifier::default();
        let test_state = ServiceState::Draining;
        let mut state = test_state;
        assert_eq!(false, state.can_start());
        assert_eq!(true, state.can_stop());
        assert_eq!(false, state.can_drain());
        assert_eq!(false, state.can_run());

        state = test_state;
        state.start_with_notification(Some(&mut notifier));
        assert_eq!(false, notifier.started);
        assert_eq!(state, test_state);

        state = test_state;
        state.run_with_notification(Some(&mut notifier));
        assert_eq!(false, notifier.running);
        assert_eq!(state, test_state);

        state = test_state;
        state.drain_with_notification(Some(&mut notifier));
        assert_eq!(false, notifier.draining);
        assert_eq!(state, test_state);

        state = test_state;
        state.stop_with_notification(Some(&mut notifier));
        assert_eq!(true, notifier.stopped);
        assert_eq!(state, ServiceState::Stopped);
    }

    #[test]
    fn service_state_advance_stopped() {
        let mut notifier = Notifier::default();
        let test_state = ServiceState::Stopped;
        let mut state = test_state;
        assert_eq!(false, state.can_start());
        assert_eq!(false, state.can_stop());
        assert_eq!(false, state.can_drain());
        assert_eq!(false, state.can_run());

        state = test_state;
        state.start_with_notification(Some(&mut notifier));
        assert_eq!(false, notifier.started);
        assert_eq!(state, test_state);

        state = test_state;
        state.run_with_notification(Some(&mut notifier));
        assert_eq!(false, notifier.running);
        assert_eq!(state, test_state);

        state = test_state;
        state.drain_with_notification(Some(&mut notifier));
        assert_eq!(false, notifier.draining);
        assert_eq!(state, test_state);

        state = test_state;
        state.stop_with_notification(Some(&mut notifier));
        assert_eq!(false, notifier.stopped);
        assert_eq!(state, test_state);
    }
}
