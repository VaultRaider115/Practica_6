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
            status: None,
            current_step: None,
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

pub struct ResponseStatus {
    pub current_step: usize,
    pub coils: [bool; 4],
}

impl ResponseStatus {
    pub fn parse_response(message: &str) -> Result<ResponseStatus, ParserError> {
        if message.len() != 7 {
            return Err(ParserError::InvalidMessageLen);
        }

        let mut iter = message.split_whitespace();

        let step = {
            let current_step = iter.next().ok_or(ParserError::MissingStep)?;

            if (current_step.len() != 2) || (!current_step.starts_with('P')) {
                return Err(ParserError::InvalidPrefix);
            }

            let step_str = current_step
                .strip_prefix('P')
                .ok_or(ParserError::InvalidPrefix)?;
            let result: usize = step_str.parse().map_err(|_| ParserError::InvalidStep)?;
            if result <= 7 {
                result
            } else {
                return Err(ParserError::InvalidStep);
            }
        };

        let coilst = {
            let valor = {
                let result = iter.next().ok_or(ParserError::MissingCoils)?;
                if result.len() != 4 {
                    return Err(ParserError::MissingCoils);
                } else {
                    result
                }
            };

            let mut coils = [false; 4];

            for (index, coil) in valor.chars().enumerate() {
                coils[index] = match coil {
                    '0' => false,
                    '1' => true,
                    _ => return Err(ParserError::InvalidCoils),
                };
            }

            coils
        };

        Ok(ResponseStatus {
            current_step: step,
            coils: coilst,
        })
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
    fn status_error() {
        let state = MotorState::new();
        assert_eq!(state.status().unwrap_err(), MotorError::MotorStatusUnknown);
    }

    #[test]
    fn current_step_error() {
        let state = MotorState::new();
        assert_eq!(
            state.current_step().unwrap_err(),
            MotorError::MotorCurrentStepUnknown
        );
    }

    #[test]
    fn current_step_test() {
        let mut state = MotorState::new();
        assert_eq!(state.set_current_step(3), Ok(()));
        assert_eq!(state.current_step().unwrap(), 3);
    }
}
