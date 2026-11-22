use crate::bluetooth::{MotorMode, MotorStatus};
use std::{num::TryFromIntError, time::Duration};

pub type CurrentMa = u16;
pub type ConsumptionUah = u64;

pub struct ConsumptionAccumulator {
    total: ConsumptionUah,
    remainder: u128,
}

#[derive(Debug, thiserror::Error)]
pub enum ConsumptionError {
    #[error("Consumption accumulation overflow")]
    AccumulationOverflow,

    #[error("Aritmetic accumulation overflow")]
    AritmeticOverflow,

    #[error("Error al convertir")]
    ConverError(#[from] TryFromIntError),
}

impl Default for ConsumptionAccumulator {
    fn default() -> Self {
        Self {
            total: 0,
            remainder: 0,
        }
    }
}

impl ConsumptionAccumulator {
    pub fn new() -> Self {
        ConsumptionAccumulator::default()
    }

    pub fn accumulate(
        &mut self,
        current: CurrentMa,
        elapsed: Duration,
    ) -> Result<(), ConsumptionError> {
        let numerator = elapsed
            .as_millis()
            .checked_mul(current as u128)
            .and_then(|value| value.checked_add(self.remainder))
            .ok_or(ConsumptionError::AritmeticOverflow)?;
        let (quotient, remainder) = ((numerator / 3600), numerator % 3600);
        let increment = u64::try_from(quotient)?;
        let rest = self
            .total
            .checked_add(increment)
            .ok_or(ConsumptionError::AccumulationOverflow)?;
        self.total = rest;
        self.remainder = remainder;
        Ok(())
    }

    pub fn total(&self) -> ConsumptionUah {
        self.total
    }
}

pub fn calc(mode: MotorMode, status: MotorStatus, current_step: usize) -> CurrentMa {
    match status {
        MotorStatus::Stopped => 0,
        MotorStatus::Active => match mode {
            MotorMode::Simple => 100,
            MotorMode::HalfStep => {
                if current_step % 2 == 1 {
                    200
                } else {
                    100
                }
            }
            MotorMode::DoubleStep => 200,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stopped_motor_consumes_zero() {
        assert_eq!(calc(MotorMode::Simple, MotorStatus::Stopped, 0), 0);
    }

    #[test]
    fn simple_mode_returns_100_ma() {
        assert_eq!(calc(MotorMode::Simple, MotorStatus::Active, 1), 100);
    }

    #[test]
    fn half_step_uses_current_step_parity() {
        assert_eq!(calc(MotorMode::HalfStep, MotorStatus::Active, 1), 200);
        assert_eq!(calc(MotorMode::HalfStep, MotorStatus::Active, 2), 100);
    }

    #[test]
    fn accumulator_calculates_consumption_correctly() {
        let mut acc = ConsumptionAccumulator::new();
        acc.accumulate(100, Duration::from_secs(36)).unwrap();
        assert_eq!(acc.total(), 1000);
    }

    #[test]
    fn accumulator_preserves_remainder() {
        let mut acc = ConsumptionAccumulator::new();
        for _ in 0..36 {
            acc.accumulate(100, Duration::from_secs(1)).unwrap();
        }
        assert_eq!(acc.total(), 1000)
    }

    #[test]
    fn accumulator_returns_conversion_overflow() {
        let mut acc = ConsumptionAccumulator::new();

        let result = acc.accumulate(u16::MAX, Duration::MAX);

        assert!(matches!(result, Err(ConsumptionError::ConverError(_))));
    }

    #[test]
    fn accumulator_returns_accumulation_overflow() {
        let mut acc = ConsumptionAccumulator::new();

        acc.total = u64::MAX;

        let result = acc.accumulate(100, Duration::from_secs(36));

        assert!(matches!(
            result,
            Err(ConsumptionError::AccumulationOverflow)
        ));
    }
}
