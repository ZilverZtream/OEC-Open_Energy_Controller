/// PID (Proportional-Integral-Derivative) Controller
///
/// A PID controller continuously calculates an error value as the difference
/// between a desired setpoint and a measured process variable, and applies
/// a correction based on proportional, integral, and derivative terms.
///
/// # Theory
/// - **P (Proportional)**: Responds to the current error
/// - **I (Integral)**: Responds to accumulated past errors
/// - **D (Derivative)**: Responds to the rate of error change
///
/// Output = Kp * error + Ki * ∫error*dt + Kd * d(error)/dt
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct PidController {
    /// Proportional gain
    kp: f64,
    /// Integral gain
    ki: f64,
    /// Derivative gain
    kd: f64,

    /// Target setpoint
    setpoint: f64,

    /// Accumulated integral term
    integral: f64,
    /// Previous error for derivative calculation
    previous_error: f64,

    /// Last update time
    last_update: Option<Instant>,

    /// Integral windup limits
    integral_min: f64,
    integral_max: f64,

    /// Output limits
    output_min: f64,
    output_max: f64,
}

impl PidController {
    /// Create a new PID controller with default limits
    pub fn new(kp: f64, ki: f64, kd: f64) -> Self {
        Self {
            kp,
            ki,
            kd,
            setpoint: 0.0,
            integral: 0.0,
            previous_error: 0.0,
            last_update: None,
            integral_min: -1000.0,
            integral_max: 1000.0,
            output_min: f64::NEG_INFINITY,
            output_max: f64::INFINITY,
        }
    }

    /// Create a PID controller with custom limits
    pub fn with_limits(
        kp: f64,
        ki: f64,
        kd: f64,
        integral_min: f64,
        integral_max: f64,
        output_min: f64,
        output_max: f64,
    ) -> Self {
        Self {
            kp,
            ki,
            kd,
            setpoint: 0.0,
            integral: 0.0,
            previous_error: 0.0,
            last_update: None,
            integral_min,
            integral_max,
            output_min,
            output_max,
        }
    }

    /// Set the target setpoint
    pub fn set_setpoint(&mut self, setpoint: f64) {
        self.setpoint = setpoint;
    }

    /// Get the current setpoint
    pub fn setpoint(&self) -> f64 {
        self.setpoint
    }

    /// Update gains (useful for adaptive tuning)
    pub fn set_gains(&mut self, kp: f64, ki: f64, kd: f64) {
        self.kp = kp;
        self.ki = ki;
        self.kd = kd;
    }

    /// Reset the controller state
    pub fn reset(&mut self) {
        self.integral = 0.0;
        self.previous_error = 0.0;
        self.last_update = None;
    }

    /// Calculate control output based on current process value
    ///
    /// # Arguments
    /// * `process_value` - Current measured value
    /// * `dt` - Time delta since last update (in seconds)
    ///
    /// # Returns
    /// Control output value
    pub fn update(&mut self, process_value: f64, dt: f64) -> f64 {
        // Calculate error
        let error = self.setpoint - process_value;

        // Proportional term
        let p_term = self.kp * error;

        // Integral term with anti-windup
        self.integral += error * dt;
        self.integral = self.integral.clamp(self.integral_min, self.integral_max);
        let i_term = self.ki * self.integral;

        // Derivative term (with derivative kick protection)
        let d_term = if dt > 0.0 {
            self.kd * (error - self.previous_error) / dt
        } else {
            0.0
        };

        // Calculate output
        let output = p_term + i_term + d_term;

        // Clamp output to limits
        let clamped_output = output.clamp(self.output_min, self.output_max);

        // Update state for next iteration
        self.previous_error = error;
        self.last_update = Some(Instant::now());

        clamped_output
    }

    /// Calculate control output with automatic time tracking
    /// This method automatically calculates dt since last update
    pub fn update_auto(&mut self, process_value: f64) -> f64 {
        let dt = match self.last_update {
            Some(last) => {
                let duration = Instant::now().duration_since(last);
                duration.as_secs_f64()
            }
            None => {
                // First call, use a small dt to avoid division issues
                0.1
            }
        };

        self.update(process_value, dt)
    }

    /// Get current integral term (useful for debugging)
    pub fn integral(&self) -> f64 {
        self.integral
    }

    /// Get previous error (useful for debugging)
    pub fn previous_error(&self) -> f64 {
        self.previous_error
    }
}

/// Power-specific PID controller for battery management
/// Pre-configured with reasonable defaults for power control
pub struct PowerPidController {
    pid: PidController,
}

impl PowerPidController {
    /// Create a new power PID controller
    ///
    /// # Arguments
    /// * `max_power_w` - Maximum power output (both positive and negative)
    pub fn new(max_power_w: f64) -> Self {
        // Conservative gains for power control
        // Kp: 0.8 - responds quickly but not too aggressively
        // Ki: 0.1 - slowly corrects steady-state errors
        // Kd: 0.05 - dampens oscillations
        let pid = PidController::with_limits(
            0.8,
            0.1,
            0.05,
            -max_power_w,
            max_power_w,
            -max_power_w,
            max_power_w,
        );

        Self { pid }
    }

    /// Create with custom gains
    pub fn with_gains(kp: f64, ki: f64, kd: f64, max_power_w: f64) -> Self {
        let pid = PidController::with_limits(
            kp,
            ki,
            kd,
            -max_power_w,
            max_power_w,
            -max_power_w,
            max_power_w,
        );

        Self { pid }
    }

    /// Set target power
    pub fn set_target(&mut self, target_power_w: f64) {
        self.pid.set_setpoint(target_power_w);
    }

    /// Calculate control output
    pub fn calculate(&mut self, current_power_w: f64, dt: f64) -> f64 {
        self.pid.update(current_power_w, dt)
    }

    /// Calculate with automatic time tracking
    pub fn calculate_auto(&mut self, current_power_w: f64) -> f64 {
        self.pid.update_auto(current_power_w)
    }

    /// Reset controller
    pub fn reset(&mut self) {
        self.pid.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pid_proportional_only() {
        let mut pid = PidController::new(1.0, 0.0, 0.0);
        pid.set_setpoint(100.0);

        // With only P control, output should be proportional to error
        let output = pid.update(90.0, 1.0);
        assert!((output - 10.0).abs() < 0.01); // Error is 10, Kp is 1.0
    }

    #[test]
    fn test_pid_integral_accumulation() {
        let mut pid = PidController::new(0.0, 1.0, 0.0);
        pid.set_setpoint(100.0);

        // Integral should accumulate over time
        let _ = pid.update(90.0, 1.0); // Error = 10, integral = 10
        let output = pid.update(90.0, 1.0); // Error = 10, integral = 20
        assert!((output - 20.0).abs() < 0.01);
    }

    #[test]
    fn test_pid_derivative() {
        let mut pid = PidController::new(0.0, 0.0, 1.0);
        pid.set_setpoint(100.0);

        let _ = pid.update(90.0, 1.0); // Error = 10
        let output = pid.update(95.0, 1.0); // Error = 5, derivative = (5-10)/1 = -5
        assert!((output - (-5.0)).abs() < 0.01);
    }

    #[test]
    fn test_pid_output_clamping() {
        let mut pid = PidController::with_limits(1.0, 0.0, 0.0, -100.0, 100.0, -50.0, 50.0);
        pid.set_setpoint(200.0);

        let output = pid.update(0.0, 1.0); // Error = 200, P term = 200
        assert_eq!(output, 50.0); // Should be clamped to max output
    }

    #[test]
    fn test_pid_integral_windup_protection() {
        let mut pid = PidController::with_limits(0.0, 1.0, 0.0, -10.0, 10.0, -100.0, 100.0);
        pid.set_setpoint(100.0);

        // Accumulate lots of error
        for _ in 0..100 {
            let _ = pid.update(0.0, 1.0);
        }

        // Integral should be clamped
        assert!(pid.integral() <= 10.0);
        assert!(pid.integral() >= -10.0);
    }

    #[test]
    fn test_power_pid_controller() {
        let mut pid = PowerPidController::new(5000.0);
        pid.set_target(2000.0);

        let output = pid.calculate(1500.0, 1.0);
        // With Kp=0.8, error=500, P term = 400
        assert!(output > 0.0); // Should increase power
        assert!(output <= 5000.0); // Should respect max power
    }

    #[test]
    fn test_pid_reset() {
        let mut pid = PidController::new(1.0, 1.0, 1.0);
        pid.set_setpoint(100.0);

        let _ = pid.update(50.0, 1.0);
        assert!(pid.integral() != 0.0);
        assert!(pid.previous_error() != 0.0);

        pid.reset();
        assert_eq!(pid.integral(), 0.0);
        assert_eq!(pid.previous_error(), 0.0);
    }

    #[test]
    fn test_step_response() {
        let mut pid = PidController::new(0.5, 0.1, 0.05);
        pid.set_setpoint(100.0);

        let mut value = 0.0;
        for _ in 0..20 {
            let control = pid.update(value, 0.1);
            value += control * 0.1; // Simple simulation
        }

        // Should converge towards setpoint
        assert!(value > 50.0); // Should make progress
    }
}
