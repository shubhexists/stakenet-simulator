-- Migration to update active_stake_jito_sol table structure
-- Change primary key from composite (id) to single (epoch)
-- Remove id and day columns, keep only epoch and balance

-- First, drop the existing primary key constraint
ALTER TABLE "public"."active_stake_jito_sol" DROP CONSTRAINT "active_stake_jito_sol_pkey";

-- Drop the unique index
DROP INDEX IF EXISTS "public"."active_stake_jito_sol_pkey";

-- Remove the id and day columns
ALTER TABLE "public"."active_stake_jito_sol" DROP COLUMN "id";
ALTER TABLE "public"."active_stake_jito_sol" DROP COLUMN "day";

-- Create new primary key on epoch column
ALTER TABLE "public"."active_stake_jito_sol" ADD CONSTRAINT "active_stake_jito_sol_pkey" PRIMARY KEY ("epoch"); 

-- Migration to update inactive_stake_jito_sol table structure

-- First, drop the existing primary key constraint
ALTER TABLE "public"."inactive_stake_jito_sol" DROP CONSTRAINT "inactive_stake_jito_sol_pkey";

-- Drop the unique index
DROP INDEX IF EXISTS "public"."inactive_stake_jito_sol_pkey";

-- Remove the id and day columns
ALTER TABLE "public"."inactive_stake_jito_sol" DROP COLUMN "id";
ALTER TABLE "public"."inactive_stake_jito_sol" DROP COLUMN "day";

-- Create new primary key on epoch column
ALTER TABLE "public"."inactive_stake_jito_sol" ADD CONSTRAINT "inactive_stake_jito_sol_pkey" PRIMARY KEY ("epoch"); 