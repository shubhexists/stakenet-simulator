create table "public"."inactive_stake_jito_sol" (
    "id" character varying(50) not null,
    "epoch" u_64 not null,
    "day" character varying(10) not null,
    "balance" numeric(20,9) not null
);


alter table "public"."inactive_stake_jito_sol" enable row level security;

CREATE UNIQUE INDEX inactive_stake_jito_sol_pkey ON public.inactive_stake_jito_sol USING btree (id);

alter table "public"."inactive_stake_jito_sol" add constraint "inactive_stake_jito_sol_pkey" PRIMARY KEY using index "inactive_stake_jito_sol_pkey";

grant delete on table "public"."inactive_stake_jito_sol" to "anon";

grant insert on table "public"."inactive_stake_jito_sol" to "anon";

grant references on table "public"."inactive_stake_jito_sol" to "anon";

grant select on table "public"."inactive_stake_jito_sol" to "anon";

grant trigger on table "public"."inactive_stake_jito_sol" to "anon";

grant truncate on table "public"."inactive_stake_jito_sol" to "anon";

grant update on table "public"."inactive_stake_jito_sol" to "anon";

grant delete on table "public"."inactive_stake_jito_sol" to "authenticated";

grant insert on table "public"."inactive_stake_jito_sol" to "authenticated";

grant references on table "public"."inactive_stake_jito_sol" to "authenticated";

grant select on table "public"."inactive_stake_jito_sol" to "authenticated";

grant trigger on table "public"."inactive_stake_jito_sol" to "authenticated";

grant truncate on table "public"."inactive_stake_jito_sol" to "authenticated";

grant update on table "public"."inactive_stake_jito_sol" to "authenticated";

grant delete on table "public"."inactive_stake_jito_sol" to "service_role";

grant insert on table "public"."inactive_stake_jito_sol" to "service_role";

grant references on table "public"."inactive_stake_jito_sol" to "service_role";

grant select on table "public"."inactive_stake_jito_sol" to "service_role";

grant trigger on table "public"."inactive_stake_jito_sol" to "service_role";

grant truncate on table "public"."inactive_stake_jito_sol" to "service_role";

grant update on table "public"."inactive_stake_jito_sol" to "service_role";

create policy "Enable read access for all users"
on "public"."inactive_stake_jito_sol"
as permissive
for select
to public
using (true);

create table "public"."active_stake_jito_sol" (
    "id" character varying(50) not null,
    "epoch" u_64 not null,
    "day" character varying(10) not null,
    "balance" numeric(20,9) not null
);


alter table "public"."active_stake_jito_sol" enable row level security;

CREATE UNIQUE INDEX active_stake_jito_sol_pkey ON public.active_stake_jito_sol USING btree (id);

alter table "public"."active_stake_jito_sol" add constraint "active_stake_jito_sol_pkey" PRIMARY KEY using index "active_stake_jito_sol_pkey";

grant delete on table "public"."active_stake_jito_sol" to "anon";

grant insert on table "public"."active_stake_jito_sol" to "anon";

grant references on table "public"."active_stake_jito_sol" to "anon";

grant select on table "public"."active_stake_jito_sol" to "anon";

grant trigger on table "public"."active_stake_jito_sol" to "anon";

grant truncate on table "public"."active_stake_jito_sol" to "anon";

grant update on table "public"."active_stake_jito_sol" to "anon";

grant delete on table "public"."active_stake_jito_sol" to "authenticated";

grant insert on table "public"."active_stake_jito_sol" to "authenticated";

grant references on table "public"."active_stake_jito_sol" to "authenticated";

grant select on table "public"."active_stake_jito_sol" to "authenticated";

grant trigger on table "public"."active_stake_jito_sol" to "authenticated";

grant truncate on table "public"."active_stake_jito_sol" to "authenticated";

grant update on table "public"."active_stake_jito_sol" to "authenticated";

grant delete on table "public"."active_stake_jito_sol" to "service_role";

grant insert on table "public"."active_stake_jito_sol" to "service_role";

grant references on table "public"."active_stake_jito_sol" to "service_role";

grant select on table "public"."active_stake_jito_sol" to "service_role";

grant trigger on table "public"."active_stake_jito_sol" to "service_role";

grant truncate on table "public"."active_stake_jito_sol" to "service_role";

grant update on table "public"."active_stake_jito_sol" to "service_role";

create policy "Enable read access for all users"
on "public"."active_stake_jito_sol"
as permissive
for select
to public
using (true); 