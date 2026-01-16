-- Forecast cache table
CREATE TABLE IF NOT EXISTS forecast_cache (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    forecast_type TEXT NOT NULL,
    area TEXT,
    household_id UUID REFERENCES households(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    valid_until TIMESTAMPTZ NOT NULL,
    data_json JSONB NOT NULL,
    source TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_forecast_cache_type_area_valid
    ON forecast_cache (forecast_type, area, valid_until DESC)
    WHERE valid_until > NOW();

-- Optimization runs table
CREATE TABLE IF NOT EXISTS optimization_runs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    household_id UUID REFERENCES households(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    duration_ms BIGINT NOT NULL,
    objective TEXT NOT NULL,
    optimizer_version TEXT NOT NULL,
    constraints_json JSONB NOT NULL,
    result_json JSONB NOT NULL,
    cost_estimate_sek DOUBLE PRECISION,
    success BOOLEAN NOT NULL DEFAULT true,
    error_message TEXT
);

CREATE INDEX IF NOT EXISTS idx_optimization_runs_household_created
    ON optimization_runs (household_id, created_at DESC);

-- Weather data cache (for PV forecasting)
CREATE TABLE IF NOT EXISTS weather_data (
    id BIGSERIAL PRIMARY KEY,
    location TEXT NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,
    temperature_c DOUBLE PRECISION,
    cloud_cover_percent DOUBLE PRECISION,
    wind_speed_ms DOUBLE PRECISION,
    precipitation_mm DOUBLE PRECISION,
    solar_irradiance_wm2 DOUBLE PRECISION,
    source TEXT NOT NULL,
    fetched_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(location, timestamp, source)
);

CREATE INDEX IF NOT EXISTS idx_weather_data_location_ts
    ON weather_data (location, timestamp DESC);

-- Battery cycle tracking for degradation
CREATE TABLE IF NOT EXISTS battery_cycles (
    id BIGSERIAL PRIMARY KEY,
    device_id UUID REFERENCES devices(id) ON DELETE CASCADE,
    cycle_date DATE NOT NULL,
    equivalent_full_cycles DOUBLE PRECISION NOT NULL,
    avg_depth_of_discharge DOUBLE PRECISION,
    max_temperature_c DOUBLE PRECISION,
    total_energy_throughput_kwh DOUBLE PRECISION NOT NULL,
    UNIQUE(device_id, cycle_date)
);

CREATE INDEX IF NOT EXISTS idx_battery_cycles_device_date
    ON battery_cycles (device_id, cycle_date DESC);
