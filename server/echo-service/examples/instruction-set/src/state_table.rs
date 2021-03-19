use super::*;

// cov: begin-ignore-line

/// StateTable is a trivial instruction set consisting of 3 commands: Init, Start, Stop.
#[derive(Debug, SmartDefault, Copy, Clone, Eq, PartialEq, MachineImpl)]
#[allow(dead_code)]
pub enum StateTable {
    #[default]
    Init,
    Start,
    Stop,
}

// cov: end-ignore-line
