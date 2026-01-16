CREATE TABLE IF NOT EXISTS devices (
    id UUID PRIMARY KEY,
    device_type TEXT NOT NULL,
    manufacturer TEXT,
    model TEXT,
    ip INET NOT NULL,
    port INT NOT NULL,
    modbus_unit_id INT,
    config JSONB NOT NULL DEFAULT '{}'::jsonb,
    discovered_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS battery_states (
    id BIGSERIAL PRIMARY KEY,
    device_id UUID REFERENCES devices(id),
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    soc_percent DOUBLE PRECISION NOT NULL,
    power_w DOUBLE PRECISION NOT NULL,
    voltage_v DOUBLE PRECISION,
    temperature_c DOUBLE PRECISION
);
CREATE INDEX IF NOT EXISTS idx_battery_states_device_ts ON battery_states (device_id, timestamp DESC);

CREATE TABLE IF NOT EXISTS electricity_prices (
    id BIGSERIAL PRIMARY KEY,
    area TEXT NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,
    price_sek_per_kwh DOUBLE PRECISION NOT NULL,
    source TEXT NOT NULL,
    UNIQUE(area, timestamp, source)
);

CREATE TABLE IF NOT EXISTS schedules (
    id UUID PRIMARY KEY,
    device_id UUID REFERENCES devices(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    valid_from TIMESTAMPTZ NOT NULL,
    valid_until TIMESTAMPTZ NOT NULL,
    schedule_json JSONB NOT NULL,
    optimizer_version TEXT NOT NULL,
    cost_savings_estimate DOUBLE PRECISION
);
