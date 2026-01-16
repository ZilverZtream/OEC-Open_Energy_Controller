-- Migration 005: Database Enhancements
-- - Add partitioning for time-series tables (battery_states, inverter_states, ev_charger_states)
-- - Add updated_at triggers for all relevant tables
-- - Create views for common queries

-- ============================================================================
-- PART 1: PARTITIONING FOR TIME-SERIES TABLES
-- ============================================================================

-- Create partitioned battery_states table (monthly partitions)
-- Note: This requires recreating the table, so we'll rename the old one first

-- Rename existing battery_states table
ALTER TABLE IF EXISTS battery_states RENAME TO battery_states_old;

-- Create new partitioned table
CREATE TABLE battery_states (
    id BIGSERIAL,
    device_id UUID NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    soc_percent DOUBLE PRECISION NOT NULL,
    power_w DOUBLE PRECISION NOT NULL,
    voltage_v DOUBLE PRECISION,
    temperature_c DOUBLE PRECISION,
    health_percent DOUBLE PRECISION,
    status TEXT,
    PRIMARY KEY (device_id, timestamp, id)
) PARTITION BY RANGE (timestamp);

-- Create index on partitioned table
CREATE INDEX idx_battery_states_device_ts ON battery_states (device_id, timestamp DESC);
CREATE INDEX idx_battery_states_timestamp ON battery_states (timestamp DESC);

-- Create foreign key (note: partitioned tables have limitations with FKs in some PG versions)
-- We'll add it as a trigger-based check instead for better compatibility
CREATE OR REPLACE FUNCTION check_battery_states_device_fk()
RETURNS TRIGGER AS $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM devices WHERE id = NEW.device_id) THEN
        RAISE EXCEPTION 'device_id % does not exist in devices table', NEW.device_id;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER battery_states_device_fk_trigger
    BEFORE INSERT OR UPDATE ON battery_states
    FOR EACH ROW
    EXECUTE FUNCTION check_battery_states_device_fk();

-- Create initial partitions (last 3 months, current month, next 3 months)
-- This ensures we have partitions ready for historical and future data

-- Helper function to create monthly partitions
CREATE OR REPLACE FUNCTION create_battery_states_partition(partition_date DATE)
RETURNS VOID AS $$
DECLARE
    partition_name TEXT;
    start_date DATE;
    end_date DATE;
BEGIN
    partition_name := 'battery_states_' || TO_CHAR(partition_date, 'YYYY_MM');
    start_date := DATE_TRUNC('month', partition_date);
    end_date := start_date + INTERVAL '1 month';

    EXECUTE format(
        'CREATE TABLE IF NOT EXISTS %I PARTITION OF battery_states
         FOR VALUES FROM (%L) TO (%L)',
        partition_name, start_date, end_date
    );
END;
$$ LANGUAGE plpgsql;

-- Create partitions for last 3 months, current month, and next 3 months
DO $$
DECLARE
    i INTEGER;
    partition_date DATE;
BEGIN
    FOR i IN -3..3 LOOP
        partition_date := DATE_TRUNC('month', CURRENT_DATE) + (i || ' months')::INTERVAL;
        PERFORM create_battery_states_partition(partition_date);
    END LOOP;
END $$;

-- Migrate data from old table to new partitioned table
INSERT INTO battery_states (id, device_id, timestamp, soc_percent, power_w, voltage_v, temperature_c)
SELECT id, device_id, timestamp, soc_percent, power_w, voltage_v, temperature_c
FROM battery_states_old
WHERE EXISTS (SELECT 1 FROM devices WHERE id = battery_states_old.device_id);

-- Drop old table after successful migration
DROP TABLE IF EXISTS battery_states_old;

-- ============================================================================
-- Similar partitioning for inverter_states and ev_charger_states
-- ============================================================================

-- INVERTER STATES
ALTER TABLE IF EXISTS inverter_states RENAME TO inverter_states_old;

CREATE TABLE inverter_states (
    id BIGSERIAL,
    device_id UUID NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ac_power_w DOUBLE PRECISION NOT NULL,
    dc_power_w DOUBLE PRECISION,
    efficiency_percent DOUBLE PRECISION,
    temperature_c DOUBLE PRECISION,
    mode TEXT,
    status TEXT,
    PRIMARY KEY (device_id, timestamp, id)
) PARTITION BY RANGE (timestamp);

CREATE INDEX idx_inverter_states_device_ts ON inverter_states (device_id, timestamp DESC);
CREATE INDEX idx_inverter_states_timestamp ON inverter_states (timestamp DESC);

CREATE TRIGGER inverter_states_device_fk_trigger
    BEFORE INSERT OR UPDATE ON inverter_states
    FOR EACH ROW
    EXECUTE FUNCTION check_battery_states_device_fk();

-- Create helper function for inverter partitions
CREATE OR REPLACE FUNCTION create_inverter_states_partition(partition_date DATE)
RETURNS VOID AS $$
DECLARE
    partition_name TEXT;
    start_date DATE;
    end_date DATE;
BEGIN
    partition_name := 'inverter_states_' || TO_CHAR(partition_date, 'YYYY_MM');
    start_date := DATE_TRUNC('month', partition_date);
    end_date := start_date + INTERVAL '1 month';

    EXECUTE format(
        'CREATE TABLE IF NOT EXISTS %I PARTITION OF inverter_states
         FOR VALUES FROM (%L) TO (%L)',
        partition_name, start_date, end_date
    );
END;
$$ LANGUAGE plpgsql;

-- Create inverter partitions
DO $$
DECLARE
    i INTEGER;
    partition_date DATE;
BEGIN
    FOR i IN -3..3 LOOP
        partition_date := DATE_TRUNC('month', CURRENT_DATE) + (i || ' months')::INTERVAL;
        PERFORM create_inverter_states_partition(partition_date);
    END LOOP;
END $$;

-- Migrate data
INSERT INTO inverter_states (id, device_id, timestamp, ac_power_w, dc_power_w, efficiency_percent, temperature_c, mode, status)
SELECT id, device_id, timestamp, ac_power_w, dc_power_w, efficiency_percent, temperature_c, mode, status
FROM inverter_states_old
WHERE EXISTS (SELECT 1 FROM devices WHERE id = inverter_states_old.device_id);

DROP TABLE IF EXISTS inverter_states_old;

-- EV CHARGER STATES
ALTER TABLE IF EXISTS ev_charger_states RENAME TO ev_charger_states_old;

CREATE TABLE ev_charger_states (
    id BIGSERIAL,
    device_id UUID NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    status TEXT NOT NULL,
    current_a DOUBLE PRECISION,
    power_w DOUBLE PRECISION,
    energy_delivered_kwh DOUBLE PRECISION,
    session_id UUID,
    connector_id INT,
    vehicle_connected BOOLEAN,
    PRIMARY KEY (device_id, timestamp, id)
) PARTITION BY RANGE (timestamp);

CREATE INDEX idx_ev_charger_states_device_ts ON ev_charger_states (device_id, timestamp DESC);
CREATE INDEX idx_ev_charger_states_timestamp ON ev_charger_states (timestamp DESC);
CREATE INDEX idx_ev_charger_states_session ON ev_charger_states (session_id);

CREATE TRIGGER ev_charger_states_device_fk_trigger
    BEFORE INSERT OR UPDATE ON ev_charger_states
    FOR EACH ROW
    EXECUTE FUNCTION check_battery_states_device_fk();

-- Create helper function for EV charger partitions
CREATE OR REPLACE FUNCTION create_ev_charger_states_partition(partition_date DATE)
RETURNS VOID AS $$
DECLARE
    partition_name TEXT;
    start_date DATE;
    end_date DATE;
BEGIN
    partition_name := 'ev_charger_states_' || TO_CHAR(partition_date, 'YYYY_MM');
    start_date := DATE_TRUNC('month', partition_date);
    end_date := start_date + INTERVAL '1 month';

    EXECUTE format(
        'CREATE TABLE IF NOT EXISTS %I PARTITION OF ev_charger_states
         FOR VALUES FROM (%L) TO (%L)',
        partition_name, start_date, end_date
    );
END;
$$ LANGUAGE plpgsql;

-- Create EV charger partitions
DO $$
DECLARE
    i INTEGER;
    partition_date DATE;
BEGIN
    FOR i IN -3..3 LOOP
        partition_date := DATE_TRUNC('month', CURRENT_DATE) + (i || ' months')::INTERVAL;
        PERFORM create_ev_charger_states_partition(partition_date);
    END LOOP;
END $$;

-- Migrate data
INSERT INTO ev_charger_states (id, device_id, timestamp, status, current_a, power_w, energy_delivered_kwh, session_id, connector_id, vehicle_connected)
SELECT id, device_id, timestamp, status, current_a, power_w, energy_delivered_kwh, session_id, connector_id, vehicle_connected
FROM ev_charger_states_old
WHERE EXISTS (SELECT 1 FROM devices WHERE id = ev_charger_states_old.device_id);

DROP TABLE IF EXISTS ev_charger_states_old;

-- ============================================================================
-- PART 2: AUTOMATIC PARTITION CREATION
-- ============================================================================

-- Function to automatically create next month's partitions
CREATE OR REPLACE FUNCTION create_next_month_partitions()
RETURNS VOID AS $$
DECLARE
    next_month DATE;
BEGIN
    next_month := DATE_TRUNC('month', CURRENT_DATE + INTERVAL '1 month');

    -- Create partitions for all time-series tables
    PERFORM create_battery_states_partition(next_month);
    PERFORM create_inverter_states_partition(next_month);
    PERFORM create_ev_charger_states_partition(next_month);

    RAISE NOTICE 'Created partitions for %', TO_CHAR(next_month, 'YYYY-MM');
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- PART 3: UPDATED_AT TRIGGERS
-- ============================================================================

-- Generic function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Add updated_at column to devices table
ALTER TABLE devices ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

-- Create trigger for devices table
CREATE TRIGGER update_devices_updated_at
    BEFORE UPDATE ON devices
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Add updated_at to schedules table
ALTER TABLE schedules ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

CREATE TRIGGER update_schedules_updated_at
    BEFORE UPDATE ON schedules
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Add updated_at to households table (if exists)
ALTER TABLE households ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

CREATE TRIGGER update_households_updated_at
    BEFORE UPDATE ON households
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Add updated_at to user_preferences table (if exists)
ALTER TABLE user_preferences ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

CREATE TRIGGER update_user_preferences_updated_at
    BEFORE UPDATE ON user_preferences
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- PART 4: USEFUL VIEWS FOR COMMON QUERIES
-- ============================================================================

-- View: Latest battery state for each device
CREATE OR REPLACE VIEW latest_battery_states AS
SELECT DISTINCT ON (device_id)
    bs.*,
    d.device_type,
    d.manufacturer,
    d.model
FROM battery_states bs
JOIN devices d ON bs.device_id = d.id
WHERE d.device_type = 'battery'
ORDER BY device_id, timestamp DESC;

-- View: Latest inverter state for each device
CREATE OR REPLACE VIEW latest_inverter_states AS
SELECT DISTINCT ON (device_id)
    inv.*,
    d.device_type,
    d.manufacturer,
    d.model
FROM inverter_states inv
JOIN devices d ON inv.device_id = d.id
WHERE d.device_type = 'inverter'
ORDER BY device_id, timestamp DESC;

-- View: Latest EV charger state for each device
CREATE OR REPLACE VIEW latest_ev_charger_states AS
SELECT DISTINCT ON (device_id)
    ev.*,
    d.device_type,
    d.manufacturer,
    d.model
FROM ev_charger_states ev
JOIN devices d ON ev.device_id = d.id
WHERE d.device_type = 'ev_charger'
ORDER BY device_id, timestamp DESC;

-- View: Active schedules (currently valid)
CREATE OR REPLACE VIEW active_schedules AS
SELECT
    s.*,
    d.device_type,
    d.manufacturer,
    d.model,
    d.ip
FROM schedules s
JOIN devices d ON s.device_id = d.id
WHERE NOW() BETWEEN s.valid_from AND s.valid_until
ORDER BY s.created_at DESC;

-- View: Device summary with latest states
CREATE OR REPLACE VIEW device_summary AS
SELECT
    d.id,
    d.device_type,
    d.manufacturer,
    d.model,
    d.ip,
    d.port,
    d.discovered_at,
    d.last_seen,
    d.updated_at,
    CASE
        WHEN d.last_seen > NOW() - INTERVAL '5 minutes' THEN 'online'
        WHEN d.last_seen > NOW() - INTERVAL '1 hour' THEN 'idle'
        ELSE 'offline'
    END AS connection_status,
    CASE
        WHEN d.device_type = 'battery' THEN (
            SELECT row_to_json(latest_battery_states.*)
            FROM latest_battery_states
            WHERE device_id = d.id
        )
        WHEN d.device_type = 'inverter' THEN (
            SELECT row_to_json(latest_inverter_states.*)
            FROM latest_inverter_states
            WHERE device_id = d.id
        )
        WHEN d.device_type = 'ev_charger' THEN (
            SELECT row_to_json(latest_ev_charger_states.*)
            FROM latest_ev_charger_states
            WHERE device_id = d.id
        )
    END AS latest_state
FROM devices d;

-- View: Hourly energy statistics for battery
CREATE OR REPLACE VIEW battery_hourly_stats AS
SELECT
    device_id,
    DATE_TRUNC('hour', timestamp) AS hour,
    AVG(soc_percent) AS avg_soc_percent,
    MIN(soc_percent) AS min_soc_percent,
    MAX(soc_percent) AS max_soc_percent,
    AVG(power_w) AS avg_power_w,
    MAX(power_w) AS max_charge_power_w,
    MIN(power_w) AS max_discharge_power_w,
    AVG(voltage_v) AS avg_voltage_v,
    AVG(temperature_c) AS avg_temperature_c,
    COUNT(*) AS sample_count
FROM battery_states
GROUP BY device_id, DATE_TRUNC('hour', timestamp);

-- View: Daily energy statistics
CREATE OR REPLACE VIEW battery_daily_stats AS
SELECT
    device_id,
    DATE_TRUNC('day', timestamp) AS day,
    AVG(soc_percent) AS avg_soc_percent,
    MIN(soc_percent) AS min_soc_percent,
    MAX(soc_percent) AS max_soc_percent,
    AVG(power_w) AS avg_power_w,
    MAX(power_w) AS max_charge_power_w,
    MIN(power_w) AS max_discharge_power_w,
    AVG(voltage_v) AS avg_voltage_v,
    AVG(temperature_c) AS avg_temperature_c,
    MAX(temperature_c) AS max_temperature_c,
    COUNT(*) AS sample_count
FROM battery_states
GROUP BY device_id, DATE_TRUNC('day', timestamp);

-- View: Consumption and production summary
CREATE OR REPLACE VIEW energy_daily_summary AS
SELECT
    DATE_TRUNC('day', timestamp) AS day,
    household_id,
    SUM(CASE WHEN source_table = 'consumption' THEN energy_kwh ELSE 0 END) AS total_consumption_kwh,
    SUM(CASE WHEN source_table = 'production' THEN energy_kwh ELSE 0 END) AS total_production_kwh,
    AVG(CASE WHEN source_table = 'consumption' THEN power_w ELSE NULL END) AS avg_consumption_power_w,
    AVG(CASE WHEN source_table = 'production' THEN power_w ELSE NULL END) AS avg_production_power_w
FROM (
    SELECT timestamp, household_id, power_w, energy_kwh, 'consumption' AS source_table
    FROM consumption_history
    UNION ALL
    SELECT timestamp, household_id, power_w, energy_kwh, 'production' AS source_table
    FROM production_history
) AS combined
GROUP BY DATE_TRUNC('day', timestamp), household_id;

-- View: Price statistics by hour of day
CREATE OR REPLACE VIEW electricity_price_hourly_patterns AS
SELECT
    area,
    EXTRACT(HOUR FROM timestamp) AS hour_of_day,
    AVG(price_sek_per_kwh) AS avg_price_sek_per_kwh,
    MIN(price_sek_per_kwh) AS min_price_sek_per_kwh,
    MAX(price_sek_per_kwh) AS max_price_sek_per_kwh,
    STDDEV(price_sek_per_kwh) AS stddev_price_sek_per_kwh,
    COUNT(*) AS sample_count
FROM electricity_prices
GROUP BY area, EXTRACT(HOUR FROM timestamp);

-- View: Optimization performance metrics
CREATE OR REPLACE VIEW optimization_performance_metrics AS
SELECT
    DATE_TRUNC('day', created_at) AS day,
    objective,
    COUNT(*) AS run_count,
    AVG(duration_ms) AS avg_duration_ms,
    MIN(duration_ms) AS min_duration_ms,
    MAX(duration_ms) AS max_duration_ms,
    PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY duration_ms) AS median_duration_ms,
    PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY duration_ms) AS p95_duration_ms
FROM optimization_runs
GROUP BY DATE_TRUNC('day', created_at), objective;

-- ============================================================================
-- PART 5: MAINTENANCE FUNCTIONS
-- ============================================================================

-- Function to drop old partitions (older than specified months)
CREATE OR REPLACE FUNCTION drop_old_partitions(table_name TEXT, months_to_keep INTEGER)
RETURNS VOID AS $$
DECLARE
    partition_record RECORD;
    cutoff_date DATE;
BEGIN
    cutoff_date := DATE_TRUNC('month', CURRENT_DATE - (months_to_keep || ' months')::INTERVAL);

    FOR partition_record IN
        SELECT tablename
        FROM pg_tables
        WHERE schemaname = 'public'
        AND tablename LIKE table_name || '_%'
        AND tablename < table_name || '_' || TO_CHAR(cutoff_date, 'YYYY_MM')
    LOOP
        EXECUTE format('DROP TABLE IF EXISTS %I CASCADE', partition_record.tablename);
        RAISE NOTICE 'Dropped old partition: %', partition_record.tablename;
    END LOOP;
END;
$$ LANGUAGE plpgsql;

-- Function to analyze and optimize partitions
CREATE OR REPLACE FUNCTION analyze_all_partitions()
RETURNS VOID AS $$
DECLARE
    partition_record RECORD;
BEGIN
    FOR partition_record IN
        SELECT tablename
        FROM pg_tables
        WHERE schemaname = 'public'
        AND (
            tablename LIKE 'battery_states_%'
            OR tablename LIKE 'inverter_states_%'
            OR tablename LIKE 'ev_charger_states_%'
        )
    LOOP
        EXECUTE format('ANALYZE %I', partition_record.tablename);
    END LOOP;

    RAISE NOTICE 'Analyzed all time-series partitions';
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- PART 6: COMMENTS AND DOCUMENTATION
-- ============================================================================

COMMENT ON FUNCTION create_battery_states_partition IS 'Creates a monthly partition for battery_states table';
COMMENT ON FUNCTION create_inverter_states_partition IS 'Creates a monthly partition for inverter_states table';
COMMENT ON FUNCTION create_ev_charger_states_partition IS 'Creates a monthly partition for ev_charger_states table';
COMMENT ON FUNCTION create_next_month_partitions IS 'Automatically creates partitions for next month across all time-series tables';
COMMENT ON FUNCTION update_updated_at_column IS 'Generic trigger function to update updated_at timestamp on row modification';
COMMENT ON FUNCTION drop_old_partitions IS 'Drops partitions older than specified number of months for a given table';
COMMENT ON FUNCTION analyze_all_partitions IS 'Runs ANALYZE on all time-series partitions to update query planner statistics';

COMMENT ON VIEW latest_battery_states IS 'Most recent battery state for each device';
COMMENT ON VIEW latest_inverter_states IS 'Most recent inverter state for each device';
COMMENT ON VIEW latest_ev_charger_states IS 'Most recent EV charger state for each device';
COMMENT ON VIEW active_schedules IS 'Currently valid schedules (between valid_from and valid_until)';
COMMENT ON VIEW device_summary IS 'Comprehensive device overview with connection status and latest state';
COMMENT ON VIEW battery_hourly_stats IS 'Hourly aggregated battery statistics';
COMMENT ON VIEW battery_daily_stats IS 'Daily aggregated battery statistics';
COMMENT ON VIEW energy_daily_summary IS 'Daily energy consumption and production summary by household';
COMMENT ON VIEW electricity_price_hourly_patterns IS 'Hourly price patterns by area for forecasting';
COMMENT ON VIEW optimization_performance_metrics IS 'Performance metrics for optimization runs';

-- Create indexes for better view performance
CREATE INDEX IF NOT EXISTS idx_consumption_history_timestamp ON consumption_history (timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_consumption_history_household ON consumption_history (household_id, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_production_history_timestamp ON production_history (timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_production_history_household ON production_history (household_id, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_electricity_prices_area_timestamp ON electricity_prices (area, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_optimization_runs_created_at ON optimization_runs (created_at DESC);
CREATE INDEX IF NOT EXISTS idx_schedules_validity ON schedules (valid_from, valid_until);

-- ============================================================================
-- END OF MIGRATION 005
-- ============================================================================
