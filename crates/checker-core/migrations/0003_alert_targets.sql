-- Free-form alert target: disk mount point, service name or port.
ALTER TABLE alert_rules ADD COLUMN target TEXT;
