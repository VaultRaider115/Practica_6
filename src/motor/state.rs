use crate::bluetooth::{MotorMode, MotorStatus};

pub struct MotorState {
    mode: Option<MotorMode>,
    status: Option<MotorStatus>,
    current_step: Option<usize>,
    coils: Option<[bool; 4]>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum MotorError {
    #[error("No se conoce el modo del motor")]
    MotorModeUnknown,

    #[error("No se conoce el estado del motor")]
    MotorStatusUnknown,

    #[error("No se conoce la fase actual del motor")]
    MotorCurrentStepUnknown,

    #[error("Se necesita una catidad menor o igual a 7")]
    InvalidCurrentStep,

    #[error("No se reportaron suficientes coils")]
    InvalidCoils,
}

impl Default for MotorState {
    fn default() -> Self {
        Self {
            mode: None,
            status: None,
            current_step: None,
            coils: Some([false; 4]),
        }
    }
}

impl MotorState {
    pub fn new() -> Self {
        MotorState {
            mode: None,
            status: Some(MotorStatus::Stopped),
            current_step: Some(0),
            coils: Some([false; 4]),
        }
    }

    pub fn mode(&self) -> Result<MotorMode, MotorError> {
        self.mode.ok_or(MotorError::MotorModeUnknown)
    }

    pub fn set_mode(&mut self, mode: MotorMode) {
        self.mode = Some(mode);
    }

    pub fn status(&self) -> Result<MotorStatus, MotorError> {
        self.status.ok_or(MotorError::MotorStatusUnknown)
    }

    pub fn set_status(&mut self, status: MotorStatus) {
        self.status = Some(status);
    }

    pub fn current_step(&self) -> Result<usize, MotorError> {
        self.current_step.ok_or(MotorError::MotorCurrentStepUnknown)
    }

    pub fn coils(&self) -> Result<[bool; 4], MotorError> {
        self.coils.ok_or(MotorError::InvalidCoils)
    }

    pub fn set_coils(&mut self, coils: [bool; 4]) {
        self.coils = Some(coils);
    }

    pub fn set_current_step(&mut self, current: usize) -> Result<(), MotorError> {
        if current <= 7 {
            self.current_step = Some(current);
            Ok(())
        } else {
            Err(MotorError::InvalidCurrentStep)
        }
    }

    pub fn clear_mode(&mut self) {
        self.mode = None;
    }

    pub fn is_initialized(&self) -> bool {
        self.status.is_some() && self.current_step.is_some()
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ParserError {
    #[error("El prefijo que se entrego es invalido")]
    InvalidPrefix,

    #[error("Se salto un paso segun la entrega")]
    MissingStep,

    #[error("El paso es invalido o es mayor de los esperado")]
    InvalidStep,

    #[error("Faltaron bobinas por entregar el reporte")]
    MissingCoils,

    #[error("Solo se admiten 1 y 0")]
    InvalidCoils,

    #[error("El mesnage no contiene los datos correctos")]
    InvalidMessageLen,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ResponseStatus {
    pub current_step: usize,
    pub coils: [bool; 4],
}

impl ResponseStatus {
    pub fn parse_response(message: &str) -> Result<ResponseStatus, ParserError> {
        let mut iter = message.split_whitespace();

        let current_step = iter.next().ok_or(ParserError::MissingStep)?;

        if current_step.len() != 2 || !current_step.starts_with('P') {
            return Err(ParserError::InvalidPrefix);
        }

        let step_str = current_step
            .strip_prefix('P')
            .ok_or(ParserError::InvalidPrefix)?;

        let step: usize = step_str.parse().map_err(|_| ParserError::InvalidStep)?;

        if step > 7 {
            return Err(ParserError::InvalidStep);
        }

        let valor = iter.next().ok_or(ParserError::MissingCoils)?;

        if valor.len() != 4 {
            return Err(ParserError::MissingCoils);
        }

        let mut coils = [false; 4];

        for (index, coil) in valor.chars().enumerate() {
            coils[index] = match coil {
                '0' => false,
                '1' => true,
                _ => return Err(ParserError::InvalidCoils),
            };
        }

        // No se permiten campos adicionales.
        if iter.next().is_some() {
            return Err(ParserError::InvalidMessageLen);
        }

        Ok(ResponseStatus {
            current_step: step,
            coils,
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ArduinoMessage {
    Step(ResponseStatus), // "P3 0110"
    Done,                 // el giro terminó completo
    Stopped,              // se detuvo con 'x'
}

impl ArduinoMessage {
    pub fn parse(message: &str) -> Result<Self, ParserError> {
        match message.trim() {
            "DONE" => Ok(Self::Done),
            "STOPPED" => Ok(Self::Stopped),
            _ => ResponseStatus::parse_response(message).map(Self::Step),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn mode_error() {
        let state = MotorState::new();
        assert_eq!(state.mode().unwrap_err(), MotorError::MotorModeUnknown);
    }

    #[test]
    fn status_defaults_to_stopped() {
        let state = MotorState::new();
        assert!(matches!(state.status(), Ok(MotorStatus::Stopped)));
    }

    #[test]
    fn current_step_defaults_to_zero() {
        let state = MotorState::new();
        assert_eq!(state.current_step().unwrap(), 0);
    }

    #[test]
    fn current_step_test() {
        let mut state = MotorState::new();
        assert_eq!(state.set_current_step(3), Ok(()));
        assert_eq!(state.current_step().unwrap(), 3);
    }

    #[test]
    fn parses_arduino_lines() {
        assert_eq!(ArduinoMessage::parse("DONE\r\n"), Ok(ArduinoMessage::Done));
        assert_eq!(
            ArduinoMessage::parse("STOPPED\n"),
            Ok(ArduinoMessage::Stopped)
        );
        assert_eq!(
            ArduinoMessage::parse("P3 0110\r\n"),
            Ok(ArduinoMessage::Step(ResponseStatus {
                current_step: 3,
                coils: [false, true, true, false],
            }))
        );
        assert_eq!(
            ArduinoMessage::parse("DONE x"),
            Err(ParserError::InvalidPrefix)
        );
    }
}
