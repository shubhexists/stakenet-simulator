--
-- Inactive Stake Jito SOL Table
--
-- This table stores inactive stake data for Jito SOL
--
CREATE TABLE IF NOT EXISTS "public"."inactive_stake_jito_sol"(
    "epoch" "public"."u_64" NOT NULL PRIMARY KEY,
    "balance" NUMERIC(20, 9) NOT NULL
);

--
-- Row Level Security Policies
--
ALTER TABLE "public"."inactive_stake_jito_sol" ENABLE ROW LEVEL SECURITY;

-- Policy: Enable read access for all users
CREATE POLICY "Enable read access for all users" ON "public"."inactive_stake_jito_sol"
    FOR SELECT
        USING (TRUE); 