-- EV Charger states table
CREATE TABLE IF NOT EXISTS ev_charger_states (
    id BIGSERIAL PRIMARY KEY,
    device_id UUID REFERENCES devices(id) ON DELETE CASCADE,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    status TEXT NOT NULL,
    connected BOOLEAN NOT NULL,
    charging BOOLEAN NOT NULL,
    current_amps DOUBLE PRECISION NOT NULL,
    power_w DOUBLE PRECISION NOT NULL,
    energy_delivered_kwh DOUBLE PRECISION NOT NULL,
    session_duration_seconds BIGINT NOT NULL,
    vehicle_soc_percent DOUBLE PRECISION
);

CREATE INDEX IF NOT EXISTS idx_ev_charger_states_device_ts
    ON ev_charger_states (device_id, timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_ev_charger_states_timestamp
    ON ev_charger_states (timestamp DESC);

-- Inverter states table
CREATE TABLE IF NOT EXISTS inverter_states (
    id BIGSERIAL PRIMARY KEY,
    device_id UUID REFERENCES devices(id) ON DELETE CASCADE,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    mode TEXT NOT NULL,
    pv_power_w DOUBLE PRECISION NOT NULL,
    ac_output_power_w DOUBLE PRECISION NOT NULL,
    dc_input_power_w DOUBLE PRECISION NOT NULL,
    grid_frequency_hz DOUBLE PRECISION NOT NULL,
    ac_voltage_v DOUBLE PRECISION NOT NULL,
    dc_voltage_v DOUBLE PRECISION NOT NULL,
    temperature_c DOUBLE PRECISION NOT NULL,
    efficiency_percent DOUBLE PRECISION NOT NULL,
    status TEXT NOT NULL,
    daily_energy_kwh DOUBLE PRECISION NOT NULL,
    total_energy_kwh DOUBLE PRECISION NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_inverter_states_device_ts
    ON inverter_states (device_id, timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_inverter_states_timestamp
    ON inverter_states (timestamp DESC);
