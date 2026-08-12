#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicState {
    High,
    Low,
    Neutral,
}

pub fn read_logic(value: f32) -> LogicState {
    if value > 0.0 {
        LogicState::High
    } else if value < 0.0 {
        LogicState::Low
    } else {
        LogicState::Neutral
    }
}

pub fn read_analog(value: f32, max: f32) -> f32 {
    (value.abs() / max).min(1.0)
}

fn is_true(value: f32) -> bool {
    matches!(read_logic(value), LogicState::High)
}

pub fn eval_and(a: f32, b: f32) -> f32 {
    if is_true(a) && is_true(b) { 1.0 } else { 0.0 }
}

pub fn eval_or(a: f32, b: f32) -> f32 {
    if is_true(a) || is_true(b) { 1.0 } else { 0.0 }
}

pub fn eval_not(a: f32) -> f32 {
    if is_true(a) { 0.0 } else { 1.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_tri_state_by_sign() {
        assert_eq!(read_logic(2.5), LogicState::High);
        assert_eq!(read_logic(-0.1), LogicState::Low);
        assert_eq!(read_logic(0.0), LogicState::Neutral);
    }

    #[test]
    fn analog_read_clamps_to_max() {
        assert_eq!(read_analog(0.0, 1.0), 0.0);
        assert_eq!(read_analog(0.5, 1.0), 0.5);
        assert_eq!(read_analog(5.0, 1.0), 1.0);
        assert_eq!(read_analog(-5.0, 1.0), 1.0);
    }

    #[test]
    fn and_gate_truth_table() {
        assert_eq!(eval_and(1.0, 1.0), 1.0);
        assert_eq!(eval_and(1.0, 0.0), 0.0);
        assert_eq!(eval_and(1.0, -1.0), 0.0);
        assert_eq!(eval_and(0.0, 0.0), 0.0);
    }

    #[test]
    fn or_gate_truth_table() {
        assert_eq!(eval_or(1.0, 0.0), 1.0);
        assert_eq!(eval_or(0.0, -1.0), 0.0);
        assert_eq!(eval_or(0.0, 0.0), 0.0);
    }

    #[test]
    fn not_gate_truth_table() {
        assert_eq!(eval_not(1.0), 0.0);
        assert_eq!(eval_not(0.0), 1.0);
        assert_eq!(eval_not(-1.0), 1.0);
    }
}
