-- Households table
CREATE TABLE IF NOT EXISTS households (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    location TEXT,
    grid_connection_kw DOUBLE PRECISION NOT NULL,
    fuse_rating_amps DOUBLE PRECISION NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Grid connection data
CREATE TABLE IF NOT EXISTS grid_connections (
    id BIGSERIAL PRIMARY KEY,
    household_id UUID REFERENCES households(id) ON DELETE CASCADE,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    status TEXT NOT NULL,
    import_power_w DOUBLE PRECISION NOT NULL,
    export_power_w DOUBLE PRECISION NOT NULL,
    frequency_hz DOUBLE PRECISION NOT NULL,
    voltage_v DOUBLE PRECISION NOT NULL,
    current_a DOUBLE PRECISION NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_grid_connections_household_ts
    ON grid_connections (household_id, timestamp DESC);

-- Consumption history table
CREATE TABLE IF NOT EXISTS consumption_history (
    id BIGSERIAL PRIMARY KEY,
    household_id UUID REFERENCES households(id) ON DELETE CASCADE,
    timestamp TIMESTAMPTZ NOT NULL,
    power_w DOUBLE PRECISION NOT NULL,
    energy_kwh DOUBLE PRECISION NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_consumption_history_household_ts
    ON consumption_history (household_id, timestamp DESC);

-- Production history table
CREATE TABLE IF NOT EXISTS production_history (
    id BIGSERIAL PRIMARY KEY,
    household_id UUID REFERENCES households(id) ON DELETE CASCADE,
    timestamp TIMESTAMPTZ NOT NULL,
    power_w DOUBLE PRECISION NOT NULL,
    energy_kwh DOUBLE PRECISION NOT NULL,
    source TEXT NOT NULL DEFAULT 'solar'
);

CREATE INDEX IF NOT EXISTS idx_production_history_household_ts
    ON production_history (household_id, timestamp DESC);

-- User preferences table
CREATE TABLE IF NOT EXISTS user_preferences (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    household_id UUID UNIQUE REFERENCES households(id) ON DELETE CASCADE,
    min_soc_percent DOUBLE PRECISION NOT NULL DEFAULT 20.0,
    max_soc_percent DOUBLE PRECISION NOT NULL DEFAULT 95.0,
    max_cycles_per_day INT NOT NULL DEFAULT 2,
    prefer_solar BOOLEAN NOT NULL DEFAULT true,
    v2g_enabled BOOLEAN NOT NULL DEFAULT false,
    price_optimization_enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
