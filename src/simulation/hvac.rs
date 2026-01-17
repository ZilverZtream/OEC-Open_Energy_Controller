use serde::{Deserialize, Serialize};

/// HVAC Operating Mode
///
/// Swedish heat pumps use a shuttle valve and cannot heat both
/// the house and DHW tank simultaneously. Priority order:
/// 1. HeatingHotWater (highest priority - morning showers!)
/// 2. Defrost (necessary for air heat pumps)
/// 3. HeatingHouse (normal operation)
/// 4. Idle (standby)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum HvacMode {
    /// Heating the house (normal operation)
    HeatingHouse,
    /// Heating DHW tank (blocks house heating!)
    HeatingHotWater,
    /// Defrost cycle (air heat pumps only, consumes power, negative heat)
    Defrost,
    /// Idle/standby
    Idle,
}

/// Domestic Hot Water (DHW) Tank State
///
/// Models the hot water tank that Swedish heat pumps charge.
/// Critical: When charging DHW, house heating stops!
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DhwTankState {
    /// Tank temperature (°C)
    pub temp_c: f64,
    /// Tank capacity (liters)
    pub capacity_liters: f64,
    /// Minimum temp before forced DHW heating (°C)
    pub min_temp_c: f64,
    /// Target temp for DHW (°C)
    pub target_temp_c: f64,
    /// Heat loss rate (W/K)
    pub heat_loss_rate_w_per_k: f64,
    /// Ambient temp around tank (°C)
    pub ambient_temp_c: f64,
}

impl Default for DhwTankState {
    fn default() -> Self {
        Self {
            temp_c: 50.0,
            capacity_liters: 200.0,        // Typical Swedish DHW tank
            min_temp_c: 45.0,               // Force heating below this
            target_temp_c: 55.0,            // Target for DHW
            heat_loss_rate_w_per_k: 2.0,    // Well insulated tank
            ambient_temp_c: 20.0,           // Indoor installation
        }
    }
}

impl DhwTankState {
    /// Check if DHW heating is required (priority over house heating)
    pub fn needs_heating(&self) -> bool {
        self.temp_c < self.min_temp_c
    }

    /// Check if DHW has reached target
    pub fn is_satisfied(&self) -> bool {
        self.temp_c >= self.target_temp_c
    }

    /// Step the DHW tank thermal model
    pub fn step(&mut self, dt_seconds: f64, heat_input_w: f64, hot_water_draw_liters: f64) {
        const WATER_SPECIFIC_HEAT: f64 = 4186.0; // J/(kg·K)
        const WATER_DENSITY: f64 = 1.0; // kg/L

        let dt_hours = dt_seconds / 3600.0;

        // Heat loss to ambient
        let heat_loss_w = (self.temp_c - self.ambient_temp_c) * self.heat_loss_rate_w_per_k;

        // Energy from heating (J)
        let energy_input_j = (heat_input_w - heat_loss_w) * dt_seconds;

        // Hot water draw (cold water in, hot water out)
        // Assume cold water at 10°C
        const COLD_WATER_TEMP_C: f64 = 10.0;
        let water_mass_kg = self.capacity_liters * WATER_DENSITY;

        // Energy lost due to hot water draw
        let energy_draw_j = hot_water_draw_liters * WATER_DENSITY * WATER_SPECIFIC_HEAT * (self.temp_c - COLD_WATER_TEMP_C);

        // Net energy change
        let net_energy_j = energy_input_j - energy_draw_j;

        // Temperature change
        let thermal_capacity_j_per_k = water_mass_kg * WATER_SPECIFIC_HEAT;
        let temp_change = net_energy_j / thermal_capacity_j_per_k;

        self.temp_c = (self.temp_c + temp_change).clamp(5.0, 85.0);
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ThreePhaseLoad {
    pub l1_amps: f64,
    pub l2_amps: f64,
    pub l3_amps: f64,
}

impl ThreePhaseLoad {
    pub fn new(l1: f64, l2: f64, l3: f64) -> Self {
        Self {
            l1_amps: l1,
            l2_amps: l2,
            l3_amps: l3,
        }
    }

    pub fn balanced(total_amps: f64) -> Self {
        let per_phase = total_amps / 3.0;
        Self::new(per_phase, per_phase, per_phase)
    }

    pub fn single_phase(phase: u8, amps: f64) -> Self {
        match phase {
            1 => Self::new(amps, 0.0, 0.0),
            2 => Self::new(0.0, amps, 0.0),
            3 => Self::new(0.0, 0.0, amps),
            _ => Self::new(0.0, 0.0, 0.0),
        }
    }

    pub fn total_power_kw(&self, voltage_v: f64) -> f64 {
        (self.l1_amps + self.l2_amps + self.l3_amps) * voltage_v / 1000.0
    }
}

/// Extended HVAC step result with detailed state information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HvacStepResult {
    /// Electrical load on three phases
    pub load: ThreePhaseLoad,
    /// Heat output to house (kW) - can be negative during defrost!
    pub house_heat_output_kw: f64,
    /// Current operating mode
    pub mode: HvacMode,
    /// Is motor currently in start-up surge (LRA)?
    pub in_startup_surge: bool,
}

pub trait HvacSystem: Send + Sync {
    fn step(&mut self, dt_seconds: f64, indoor_temp: f64, outdoor_temp: f64) -> (ThreePhaseLoad, f64);
    fn name(&self) -> &str;

    /// Extended step with DHW priority and full state
    fn step_extended(&mut self, dt_seconds: f64, indoor_temp: f64, outdoor_temp: f64, dhw_draw_liters: f64) -> HvacStepResult {
        // Default implementation for backward compatibility
        let (load, heat) = self.step(dt_seconds, indoor_temp, outdoor_temp);
        HvacStepResult {
            load,
            house_heat_output_kw: heat / 1000.0,
            mode: if heat > 0.0 { HvacMode::HeatingHouse } else { HvacMode::Idle },
            in_startup_surge: false,
        }
    }

    /// Get DHW tank state (if supported)
    fn dhw_tank_state(&self) -> Option<&DhwTankState> {
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeothermalHeatPumpConfig {
    pub compressor_power_kw: f64,
    pub circulation_pump_power_kw: f64,
    pub ground_loop_temp_c: f64,
    pub cop_at_nominal: f64,
    pub target_temp_c: f64,
    pub hysteresis_c: f64,
    pub nominal_voltage_v: f64,

    /// CRITICAL FIX #11: Minimum Run Time (seconds)
    ///
    /// Heat pump compressors MUST NOT cycle rapidly. Minimum run time prevents:
    /// 1. Mechanical wear on compressor start/stop cycles
    /// 2. Efficiency loss from frequent transients
    /// 3. Hardware destruction from rapid cycling
    ///
    /// Typical values:
    /// - Geothermal: 300-600 seconds (5-10 minutes)
    /// - Air source: 180-300 seconds (3-5 minutes)
    ///
    /// If simulation allows cycling every second (as indoor_temp fluctuates
    /// around target_temp), the model validates control logic that would
    /// mechanically destroy a real heat pump in days.
    pub min_run_time_seconds: f64,

    /// Minimum off time before compressor can restart (seconds)
    ///
    /// After shutdown, compressor must wait for pressure equalization.
    /// Typical: 180-300 seconds (3-5 minutes)
    pub min_off_time_seconds: f64,
}

impl Default for GeothermalHeatPumpConfig {
    fn default() -> Self {
        Self {
            compressor_power_kw: 3.0,
            circulation_pump_power_kw: 0.15,
            ground_loop_temp_c: 4.0,
            cop_at_nominal: 4.5,
            target_temp_c: 21.0,
            hysteresis_c: 1.0,
            nominal_voltage_v: 230.0,
            // Enforce minimum 5 minute run time (300 seconds)
            min_run_time_seconds: 300.0,
            // Enforce minimum 3 minute off time (180 seconds)
            min_off_time_seconds: 180.0,
        }
    }
}

pub struct GeothermalHeatPump {
    config: GeothermalHeatPumpConfig,
    is_running: bool,
    /// DHW tank state
    dhw_tank: Option<DhwTankState>,
    /// Current operating mode
    mode: HvacMode,
    /// Time in current mode (seconds)
    time_in_mode: f64,
    /// Motor start-up surge tracker (Locked Rotor Amps)
    /// Surge lasts ~200ms at 5-7x rated current
    startup_surge_remaining: f64,
}

impl GeothermalHeatPump {
    pub fn new(config: GeothermalHeatPumpConfig) -> Self {
        Self {
            config,
            is_running: false,
            dhw_tank: None,
            mode: HvacMode::Idle,
            time_in_mode: 0.0,
            startup_surge_remaining: 0.0,
        }
    }

    pub fn with_dhw_tank(config: GeothermalHeatPumpConfig, dhw_tank: DhwTankState) -> Self {
        Self {
            config,
            is_running: false,
            dhw_tank: Some(dhw_tank),
            mode: HvacMode::Idle,
            time_in_mode: 0.0,
            startup_surge_remaining: 0.0,
        }
    }
}

impl HvacSystem for GeothermalHeatPump {
    fn step(&mut self, _dt_seconds: f64, indoor_temp: f64, _outdoor_temp: f64) -> (ThreePhaseLoad, f64) {
        if self.is_running {
            if indoor_temp > self.config.target_temp_c + self.config.hysteresis_c {
                self.is_running = false;
            }
        } else {
            if indoor_temp < self.config.target_temp_c - self.config.hysteresis_c {
                self.is_running = true;
            }
        }

        if self.is_running {
            let total_power = self.config.compressor_power_kw + self.config.circulation_pump_power_kw;
            let compressor_current = (self.config.compressor_power_kw * 1000.0) / self.config.nominal_voltage_v;
            let pump_current = (self.config.circulation_pump_power_kw * 1000.0) / self.config.nominal_voltage_v;

            let load = ThreePhaseLoad::new(
                compressor_current / 3.0 + pump_current,
                compressor_current / 3.0,
                compressor_current / 3.0,
            );

            let heat_output = total_power * self.config.cop_at_nominal;

            (load, heat_output)
        } else {
            let idle_power = self.config.circulation_pump_power_kw * 0.3;
            let idle_current = (idle_power * 1000.0) / self.config.nominal_voltage_v;
            let load = ThreePhaseLoad::single_phase(1, idle_current);

            (load, 0.0)
        }
    }

    fn step_extended(&mut self, dt_seconds: f64, indoor_temp: f64, outdoor_temp: f64, dhw_draw_liters: f64) -> HvacStepResult {
        const MOTOR_STARTUP_SURGE_DURATION: f64 = 0.2; // 200ms
        const MOTOR_STARTUP_SURGE_MULTIPLIER: f64 = 6.0; // 6x rated current (LRA)

        self.time_in_mode += dt_seconds;

        // Determine desired mode based on priorities
        let desired_mode = if let Some(ref dhw_tank) = self.dhw_tank {
            if dhw_tank.needs_heating() {
                HvacMode::HeatingHotWater
            } else if indoor_temp < self.config.target_temp_c - self.config.hysteresis_c {
                HvacMode::HeatingHouse
            } else if indoor_temp > self.config.target_temp_c + self.config.hysteresis_c {
                HvacMode::Idle
            } else {
                self.mode // Maintain current mode in hysteresis band
            }
        } else {
            // No DHW tank, simple house heating
            if indoor_temp < self.config.target_temp_c - self.config.hysteresis_c {
                HvacMode::HeatingHouse
            } else if indoor_temp > self.config.target_temp_c + self.config.hysteresis_c {
                HvacMode::Idle
            } else {
                self.mode
            }
        };

        // CRITICAL FIX #11: Enforce minimum run time to prevent bang-bang cycling
        // Don't allow mode changes if minimum run time hasn't been met
        let can_change_mode = if self.mode == HvacMode::Idle {
            // If idle, check minimum off time before starting
            self.time_in_mode >= self.config.min_off_time_seconds
        } else {
            // If running (HeatingHouse or HeatingHotWater), check minimum run time before stopping
            if desired_mode == HvacMode::Idle {
                self.time_in_mode >= self.config.min_run_time_seconds
            } else {
                // Allow switching between HeatingHouse and HeatingHotWater
                // (DHW priority can override house heating immediately)
                true
            }
        };

        // Detect mode change and trigger startup surge (only if minimum times are met)
        if can_change_mode && desired_mode != self.mode && desired_mode != HvacMode::Idle {
            // Starting compressor - trigger LRA surge
            self.startup_surge_remaining = MOTOR_STARTUP_SURGE_DURATION;
            self.mode = desired_mode;
            self.time_in_mode = 0.0;
        } else if can_change_mode && desired_mode == HvacMode::Idle && self.mode != HvacMode::Idle {
            // Shutting down (only if minimum run time met)
            self.mode = HvacMode::Idle;
            self.time_in_mode = 0.0;
            self.startup_surge_remaining = 0.0;
        }
        // else: maintain current mode until minimum time is met

        // Update startup surge timer
        if self.startup_surge_remaining > 0.0 {
            self.startup_surge_remaining -= dt_seconds;
            if self.startup_surge_remaining < 0.0 {
                self.startup_surge_remaining = 0.0;
            }
        }

        let in_startup_surge = self.startup_surge_remaining > 0.0;

        // Calculate power and heat based on mode
        let (load, house_heat_output_kw, dhw_heat_output_w) = match self.mode {
            HvacMode::HeatingHouse => {
                let total_power = self.config.compressor_power_kw + self.config.circulation_pump_power_kw;
                let mut compressor_current = (self.config.compressor_power_kw * 1000.0) / self.config.nominal_voltage_v;
                let pump_current = (self.config.circulation_pump_power_kw * 1000.0) / self.config.nominal_voltage_v;

                // Apply startup surge multiplier
                if in_startup_surge {
                    compressor_current *= MOTOR_STARTUP_SURGE_MULTIPLIER;
                }

                let load = ThreePhaseLoad::new(
                    compressor_current / 3.0 + pump_current,
                    compressor_current / 3.0,
                    compressor_current / 3.0,
                );

                let heat_output_kw = total_power * self.config.cop_at_nominal;
                (load, heat_output_kw, 0.0)
            }
            HvacMode::HeatingHotWater => {
                // DHW mode: heat goes to tank, NOT to house
                let total_power = self.config.compressor_power_kw + self.config.circulation_pump_power_kw;
                let mut compressor_current = (self.config.compressor_power_kw * 1000.0) / self.config.nominal_voltage_v;
                let pump_current = (self.config.circulation_pump_power_kw * 1000.0) / self.config.nominal_voltage_v;

                if in_startup_surge {
                    compressor_current *= MOTOR_STARTUP_SURGE_MULTIPLIER;
                }

                let load = ThreePhaseLoad::new(
                    compressor_current / 3.0 + pump_current,
                    compressor_current / 3.0,
                    compressor_current / 3.0,
                );

                // Heat goes to DHW tank, house gets ZERO
                let heat_output_kw = total_power * self.config.cop_at_nominal;
                let dhw_heat_w = heat_output_kw * 1000.0;
                (load, 0.0, dhw_heat_w)
            }
            HvacMode::Idle | HvacMode::Defrost => {
                let idle_power = self.config.circulation_pump_power_kw * 0.3;
                let idle_current = (idle_power * 1000.0) / self.config.nominal_voltage_v;
                let load = ThreePhaseLoad::single_phase(1, idle_current);
                (load, 0.0, 0.0)
            }
        };

        // Update DHW tank
        if let Some(ref mut dhw_tank) = self.dhw_tank {
            dhw_tank.step(dt_seconds, dhw_heat_output_w, dhw_draw_liters);
        }

        HvacStepResult {
            load,
            house_heat_output_kw,
            mode: self.mode,
            in_startup_surge,
        }
    }

    fn dhw_tank_state(&self) -> Option<&DhwTankState> {
        self.dhw_tank.as_ref()
    }

    fn name(&self) -> &str {
        "Geothermal Heat Pump (Bergvärme)"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AirHeatPumpConfig {
    pub nominal_power_kw: f64,
    pub target_temp_c: f64,
    pub hysteresis_c: f64,
    pub nominal_voltage_v: f64,
    pub electric_element_power_kw: f64,
    pub electric_element_threshold_c: f64,
}

impl Default for AirHeatPumpConfig {
    fn default() -> Self {
        Self {
            nominal_power_kw: 2.5,
            target_temp_c: 21.0,
            hysteresis_c: 1.0,
            nominal_voltage_v: 230.0,
            electric_element_power_kw: 5.0,
            electric_element_threshold_c: -15.0,
        }
    }
}

pub struct AirHeatPump {
    config: AirHeatPumpConfig,
    is_running: bool,
    /// DHW tank state
    dhw_tank: Option<DhwTankState>,
    /// Current operating mode
    mode: HvacMode,
    /// Time in current mode (seconds)
    time_in_mode: f64,
    /// Motor start-up surge tracker
    startup_surge_remaining: f64,
    /// Time since last defrost (seconds)
    time_since_defrost: f64,
    /// Currently in defrost cycle
    in_defrost: bool,
}

impl AirHeatPump {
    pub fn new(config: AirHeatPumpConfig) -> Self {
        Self {
            config,
            is_running: false,
            dhw_tank: None,
            mode: HvacMode::Idle,
            time_in_mode: 0.0,
            startup_surge_remaining: 0.0,
            time_since_defrost: 0.0,
            in_defrost: false,
        }
    }

    pub fn with_dhw_tank(config: AirHeatPumpConfig, dhw_tank: DhwTankState) -> Self {
        Self {
            config,
            is_running: false,
            dhw_tank: Some(dhw_tank),
            mode: HvacMode::Idle,
            time_in_mode: 0.0,
            startup_surge_remaining: 0.0,
            time_since_defrost: 0.0,
            in_defrost: false,
        }
    }

    fn calculate_cop(&self, outdoor_temp: f64) -> f64 {
        if outdoor_temp >= 7.0 {
            4.2
        } else if outdoor_temp >= 0.0 {
            3.5 + (outdoor_temp / 7.0) * 0.7
        } else if outdoor_temp >= -10.0 {
            2.5 + ((outdoor_temp + 10.0) / 10.0) * 1.0
        } else if outdoor_temp >= -20.0 {
            1.5 + ((outdoor_temp + 20.0) / 10.0) * 1.0
        } else {
            1.2
        }
    }

    /// Check if defrost cycle is needed
    /// Critical for Swedish winter: Between +2°C and -5°C with high humidity,
    /// ice builds up on outdoor coil and must be melted every 45-90 mins
    fn needs_defrost(&self, outdoor_temp: f64) -> bool {
        // Defrost zone: +2°C to -5°C (wet winter conditions)
        const DEFROST_TEMP_HIGH: f64 = 2.0;
        const DEFROST_TEMP_LOW: f64 = -5.0;
        const DEFROST_INTERVAL: f64 = 2700.0; // 45 minutes

        if outdoor_temp > DEFROST_TEMP_HIGH || outdoor_temp < DEFROST_TEMP_LOW {
            // Outside defrost zone
            return false;
        }

        // In defrost zone - check if enough time has passed
        self.time_since_defrost >= DEFROST_INTERVAL && self.is_running
    }
}

impl HvacSystem for AirHeatPump {
    fn step(&mut self, _dt_seconds: f64, indoor_temp: f64, outdoor_temp: f64) -> (ThreePhaseLoad, f64) {
        if self.is_running {
            if indoor_temp > self.config.target_temp_c + self.config.hysteresis_c {
                self.is_running = false;
            }
        } else {
            if indoor_temp < self.config.target_temp_c - self.config.hysteresis_c {
                self.is_running = true;
            }
        }

        if self.is_running {
            let cop = self.calculate_cop(outdoor_temp);
            let mut power_draw = self.config.nominal_power_kw;
            let mut heat_output = power_draw * cop;

            if outdoor_temp < self.config.electric_element_threshold_c {
                power_draw += self.config.electric_element_power_kw;
                heat_output += self.config.electric_element_power_kw;
            }

            let current = (power_draw * 1000.0) / self.config.nominal_voltage_v;
            let load = ThreePhaseLoad::single_phase(1, current);

            (load, heat_output)
        } else {
            let load = ThreePhaseLoad::new(0.0, 0.0, 0.0);
            (load, 0.0)
        }
    }

    fn step_extended(&mut self, dt_seconds: f64, indoor_temp: f64, outdoor_temp: f64, dhw_draw_liters: f64) -> HvacStepResult {
        const MOTOR_STARTUP_SURGE_DURATION: f64 = 0.2;
        const MOTOR_STARTUP_SURGE_MULTIPLIER: f64 = 6.0;
        const DEFROST_DURATION: f64 = 300.0; // 5 minutes defrost cycle
        const DEFROST_POWER_KW: f64 = 1.5; // Power consumed during defrost

        self.time_in_mode += dt_seconds;
        self.time_since_defrost += dt_seconds;

        // Check if defrost is needed
        if self.needs_defrost(outdoor_temp) && !self.in_defrost {
            // Start defrost cycle
            self.in_defrost = true;
            self.mode = HvacMode::Defrost;
            self.time_in_mode = 0.0;
            self.startup_surge_remaining = MOTOR_STARTUP_SURGE_DURATION;
        }

        // Check if defrost is complete
        if self.in_defrost && self.time_in_mode >= DEFROST_DURATION {
            self.in_defrost = false;
            self.time_since_defrost = 0.0;
            self.mode = HvacMode::Idle;
        }

        // Determine desired mode (if not in defrost)
        let desired_mode = if self.in_defrost {
            HvacMode::Defrost
        } else if let Some(ref dhw_tank) = self.dhw_tank {
            if dhw_tank.needs_heating() {
                HvacMode::HeatingHotWater
            } else if indoor_temp < self.config.target_temp_c - self.config.hysteresis_c {
                HvacMode::HeatingHouse
            } else if indoor_temp > self.config.target_temp_c + self.config.hysteresis_c {
                HvacMode::Idle
            } else {
                self.mode
            }
        } else {
            if indoor_temp < self.config.target_temp_c - self.config.hysteresis_c {
                HvacMode::HeatingHouse
            } else if indoor_temp > self.config.target_temp_c + self.config.hysteresis_c {
                HvacMode::Idle
            } else {
                self.mode
            }
        };

        // Detect mode change and trigger startup surge
        if desired_mode != self.mode && desired_mode != HvacMode::Idle && !self.in_defrost {
            self.startup_surge_remaining = MOTOR_STARTUP_SURGE_DURATION;
            self.mode = desired_mode;
            self.time_in_mode = 0.0;
        } else if desired_mode == HvacMode::Idle && self.mode != HvacMode::Idle && !self.in_defrost {
            self.mode = HvacMode::Idle;
            self.time_in_mode = 0.0;
            self.startup_surge_remaining = 0.0;
        }

        // Update startup surge timer
        if self.startup_surge_remaining > 0.0 {
            self.startup_surge_remaining -= dt_seconds;
            if self.startup_surge_remaining < 0.0 {
                self.startup_surge_remaining = 0.0;
            }
        }

        let in_startup_surge = self.startup_surge_remaining > 0.0;

        // Calculate power and heat based on mode
        let (load, house_heat_output_kw, dhw_heat_output_w) = match self.mode {
            HvacMode::HeatingHouse => {
                let cop = self.calculate_cop(outdoor_temp);
                let mut power_draw_kw = self.config.nominal_power_kw;
                let mut heat_output_kw = power_draw_kw * cop;

                // Electric element backup
                if outdoor_temp < self.config.electric_element_threshold_c {
                    power_draw_kw += self.config.electric_element_power_kw;
                    heat_output_kw += self.config.electric_element_power_kw;
                }

                let mut current = (power_draw_kw * 1000.0) / self.config.nominal_voltage_v;
                if in_startup_surge {
                    current *= MOTOR_STARTUP_SURGE_MULTIPLIER;
                }

                let load = ThreePhaseLoad::single_phase(1, current);
                (load, heat_output_kw, 0.0)
            }
            HvacMode::HeatingHotWater => {
                let cop = self.calculate_cop(outdoor_temp);
                let power_draw_kw = self.config.nominal_power_kw;
                let heat_output_kw = power_draw_kw * cop;

                let mut current = (power_draw_kw * 1000.0) / self.config.nominal_voltage_v;
                if in_startup_surge {
                    current *= MOTOR_STARTUP_SURGE_MULTIPLIER;
                }

                let load = ThreePhaseLoad::single_phase(1, current);
                // Heat goes to DHW, house gets ZERO
                let dhw_heat_w = heat_output_kw * 1000.0;
                (load, 0.0, dhw_heat_w)
            }
            HvacMode::Defrost => {
                // CRITICAL: Defrost consumes power but produces NEGATIVE heat
                // (reverse cycle to melt ice, cools the house)
                let mut current = (DEFROST_POWER_KW * 1000.0) / self.config.nominal_voltage_v;
                if in_startup_surge {
                    current *= MOTOR_STARTUP_SURGE_MULTIPLIER;
                }

                let load = ThreePhaseLoad::single_phase(1, current);
                // Negative heat output (cools the house by ~500W)
                (load, -0.5, 0.0)
            }
            HvacMode::Idle => {
                let load = ThreePhaseLoad::new(0.0, 0.0, 0.0);
                (load, 0.0, 0.0)
            }
        };

        // Update DHW tank
        if let Some(ref mut dhw_tank) = self.dhw_tank {
            dhw_tank.step(dt_seconds, dhw_heat_output_w, dhw_draw_liters);
        }

        HvacStepResult {
            load,
            house_heat_output_kw,
            mode: self.mode,
            in_startup_surge,
        }
    }

    fn dhw_tank_state(&self) -> Option<&DhwTankState> {
        self.dhw_tank.as_ref()
    }

    fn name(&self) -> &str {
        "Air-to-Air Heat Pump (Luftvärmepump)"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geothermal_heat_pump() {
        let config = GeothermalHeatPumpConfig::default();
        let mut hp = GeothermalHeatPump::new(config);

        let (load, heat) = hp.step(60.0, 18.0, -10.0);
        assert!(load.total_power_kw(230.0) > 0.0);
        assert!(heat > 0.0);
    }

    #[test]
    fn test_air_heat_pump_cop_variation() {
        let config = AirHeatPumpConfig::default();
        let hp = AirHeatPump::new(config);

        let cop_warm = hp.calculate_cop(7.0);
        let cop_cold = hp.calculate_cop(-10.0);
        let cop_very_cold = hp.calculate_cop(-25.0);

        assert!(cop_warm > cop_cold);
        assert!(cop_cold > cop_very_cold);
        assert!(cop_very_cold >= 1.0);
    }

    #[test]
    fn test_three_phase_balanced() {
        let load = ThreePhaseLoad::balanced(30.0);
        assert_eq!(load.l1_amps, 10.0);
        assert_eq!(load.l2_amps, 10.0);
        assert_eq!(load.l3_amps, 10.0);
    }
}
