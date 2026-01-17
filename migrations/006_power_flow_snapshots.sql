-- Power Flow Snapshot Persistence
--
-- Captures complete power flow decisions made by the BatteryController
-- for debugging, auditing, and optimization analysis.

CREATE TABLE IF NOT EXISTS power_flow_snapshots (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Power flow values (kW)
    pv_production_kw DOUBLE PRECISION NOT NULL,
    house_load_kw DOUBLE PRECISION NOT NULL,
    battery_power_kw DOUBLE PRECISION NOT NULL,
    ev_charger_power_kw DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    grid_import_kw DOUBLE PRECISION NOT NULL,
    grid_export_kw DOUBLE PRECISION NOT NULL,

    -- Battery state
    battery_soc_percent DOUBLE PRECISION,
    battery_temperature_c DOUBLE PRECISION,

    -- Grid state
    grid_frequency_hz DOUBLE PRECISION,
    grid_voltage_v DOUBLE PRECISION,
    grid_available BOOLEAN NOT NULL DEFAULT TRUE,

    -- Constraints version (for tracking which constraint set was active)
    constraints_version TEXT,
    fuse_limit_a DOUBLE PRECISION,

    -- Decision metadata
    control_mode TEXT, -- e.g., "schedule", "emergency", "manual"
    decision_reason TEXT, -- Human-readable reason for the power flow decision

    -- Economic metrics
    spot_price_sek_per_kwh DOUBLE PRECISION,
    estimated_cost_sek DOUBLE PRECISION,

    -- Optimization metrics
    schedule_id UUID REFERENCES schedules(id) ON DELETE SET NULL,
    deviation_from_schedule_kw DOUBLE PRECISION,

    CONSTRAINT valid_power_flow CHECK (
        pv_production_kw >= 0 AND
        house_load_kw >= 0 AND
        grid_import_kw >= 0 AND
        grid_export_kw >= 0 AND
        (battery_soc_percent IS NULL OR (battery_soc_percent >= 0 AND battery_soc_percent <= 100))
    )
);

-- Index for time-series queries
CREATE INDEX IF NOT EXISTS idx_power_flow_snapshots_timestamp
    ON power_flow_snapshots (timestamp DESC);

-- Index for schedule correlation
CREATE INDEX IF NOT EXISTS idx_power_flow_snapshots_schedule
    ON power_flow_snapshots (schedule_id, timestamp DESC);

-- Index for control mode analysis
CREATE INDEX IF NOT EXISTS idx_power_flow_snapshots_mode
    ON power_flow_snapshots (control_mode, timestamp DESC);

-- Materialized view for hourly aggregates (useful for dashboard queries)
CREATE MATERIALIZED VIEW IF NOT EXISTS power_flow_hourly_stats AS
SELECT
    DATE_TRUNC('hour', timestamp) AS hour,
    COUNT(*) AS snapshot_count,
    AVG(pv_production_kw) AS avg_pv_kw,
    AVG(house_load_kw) AS avg_house_load_kw,
    AVG(battery_power_kw) AS avg_battery_power_kw,
    AVG(grid_import_kw) AS avg_grid_import_kw,
    AVG(grid_export_kw) AS avg_grid_export_kw,
    SUM(grid_import_kw * 10.0 / 3600.0) AS total_grid_import_kwh, -- Assuming 10s samples
    SUM(grid_export_kw * 10.0 / 3600.0) AS total_grid_export_kwh,
    AVG(spot_price_sek_per_kwh) AS avg_spot_price,
    SUM(estimated_cost_sek) AS total_cost_sek,
    COUNT(*) FILTER (WHERE NOT grid_available) AS grid_outage_count
FROM power_flow_snapshots
GROUP BY DATE_TRUNC('hour', timestamp);

CREATE UNIQUE INDEX IF NOT EXISTS idx_power_flow_hourly_stats_hour
    ON power_flow_hourly_stats (hour DESC);

-- Function to refresh the materialized view
CREATE OR REPLACE FUNCTION refresh_power_flow_hourly_stats()
RETURNS void AS $$
BEGIN
    REFRESH MATERIALIZED VIEW CONCURRENTLY power_flow_hourly_stats;
END;
$$ LANGUAGE plpgsql;

-- Comments for documentation
COMMENT ON TABLE power_flow_snapshots IS
    'Stores complete power flow state snapshots from the BatteryController control loop (typically 10s intervals)';

COMMENT ON COLUMN power_flow_snapshots.control_mode IS
    'Control mode: schedule (following optimizer), emergency (safety override), manual (user command)';

COMMENT ON COLUMN power_flow_snapshots.decision_reason IS
    'Human-readable explanation of why this power flow was chosen (e.g., "Following schedule", "Fuse limit protection")';

COMMENT ON COLUMN power_flow_snapshots.deviation_from_schedule_kw IS
    'Absolute difference between actual battery power and scheduled battery power';

COMMENT ON MATERIALIZED VIEW power_flow_hourly_stats IS
    'Hourly aggregated statistics for power flow - refresh periodically for dashboard queries';
