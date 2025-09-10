--
-- Withdraws And Deposits Table
-- This table stores transactions of withdraws and deposits
--
CREATE TABLE
    IF NOT EXISTS public.withdraws_and_deposits (
        epoch public.u_64 NOT NULL PRIMARY KEY,
        deposit_sol NUMERIC(20, 9) DEFAULT 0,
        withdraw_stake NUMERIC(20, 9) DEFAULT 0,
        deposit_stake NUMERIC(20, 9) DEFAULT 0,
        withdraw_sol NUMERIC(20, 9) DEFAULT 0,
        total_stake NUMERIC(20, 9) DEFAULT 0
    );

-- Enable RLS
ALTER TABLE public.withdraws_and_deposits ENABLE ROW LEVEL SECURITY;

-- Grants: anon
GRANT DELETE,
INSERT,
REFERENCES,
SELECT
,
    TRIGGER,
    TRUNCATE,
UPDATE ON public.withdraws_and_deposits TO anon;

-- Grants: authenticated
GRANT DELETE,
INSERT,
REFERENCES,
SELECT
,
    TRIGGER,
    TRUNCATE,
UPDATE ON public.withdraws_and_deposits TO authenticated;

-- Grants: service_role
GRANT DELETE,
INSERT,
REFERENCES,
SELECT
,
    TRIGGER,
    TRUNCATE,
UPDATE ON public.withdraws_and_deposits TO service_role;

-- Policy: Enable read access for all users
CREATE POLICY "Enable read access for all users" ON public.withdraws_and_deposits AS PERMISSIVE FOR
SELECT
    TO public USING (TRUE);