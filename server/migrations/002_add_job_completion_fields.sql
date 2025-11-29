-- Add completion metadata to jobs.
ALTER TABLE jobs ADD COLUMN finished_at TEXT;
ALTER TABLE jobs ADD COLUMN result_json TEXT;
