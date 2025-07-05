--
-- Custom Domain Types
--
-- This file contains all custom domain definitions for the Pye Backend database.
-- Domains are custom data types that extend PostgreSQL's built-in types with specific constraints.
--
-- Create an alias type for Solana's Pubkey to be reused.
-- NOTE: the pubkey is base58 encoded, so it can be up to 44 characters long.
CREATE DOMAIN "public"."solana_pubkey" AS character varying(44);

ALTER DOMAIN "public"."solana_pubkey" OWNER TO "postgres";

-- Create an alias type for u64 to be reused.
CREATE DOMAIN "public"."u_64" AS numeric(20, 0);

ALTER DOMAIN "public"."u_64" OWNER TO "postgres";
