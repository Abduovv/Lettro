BEGIN;

-- Backfill `status` for historical entries
UPDATE subscriptions
    SET status = 'pending_confirmation'
    WHERE status IS NULL;

-- Set a default for new rows
ALTER TABLE subscriptions
    ALTER COLUMN status SET DEFAULT 'pending_confirmation';

-- Make `status` mandatory
ALTER TABLE subscriptions
    ALTER COLUMN status SET NOT NULL;

COMMIT;
