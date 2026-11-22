#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotorCommand {
    SimpleLeft,
    SimpleRight,
    HalfStepLeft,
    HalfStepRight,
    DoubleStepLeft,
    DoubleStepRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotorMode {
    Simple,
    HalfStep,
    DoubleStep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotorStatus {
    Active,
    Stopped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotorControl {
    Stop,
}

impl From<MotorCommand> for u8 {
    fn from(value: MotorCommand) -> Self {
        match value {
            MotorCommand::SimpleLeft => b'a',
            MotorCommand::SimpleRight => b'b',
            MotorCommand::HalfStepLeft => b'c',
            MotorCommand::HalfStepRight => b'd',
            MotorCommand::DoubleStepLeft => b'e',
            MotorCommand::DoubleStepRight => b'f',
        }
    }
}

impl From<MotorControl> for u8 {
    fn from(value: MotorControl) -> Self {
        match value {
            MotorControl::Stop => b'x',
        }
    }
}

impl From<MotorCommand> for MotorMode {
    fn from(value: MotorCommand) -> Self {
        match value {
            MotorCommand::SimpleLeft | MotorCommand::SimpleRight => Self::Simple,
            MotorCommand::HalfStepLeft | MotorCommand::HalfStepRight => Self::HalfStep,
            MotorCommand::DoubleStepLeft | MotorCommand::DoubleStepRight => Self::DoubleStep,
        }
    }
}
