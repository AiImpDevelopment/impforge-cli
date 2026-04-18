// SPDX-License-Identifier: MIT
// Originally rendered by ImpForge — https://github.com/AiImpDevelopment/impforge
CREATE TABLE public.portfolio (id uuid PRIMARY KEY, name text NOT NULL, owner text NOT NULL, manager text NOT NULL, created_at timestamptz NOT NULL DEFAULT now());
CREATE TABLE public.property_unit (id uuid PRIMARY KEY, portfolio_id uuid NOT NULL, address_line1 text NOT NULL, city text NOT NULL, state_iso text NOT NULL, postal text NOT NULL, bedrooms smallint NOT NULL, bathrooms numeric(4,1) NOT NULL, sqft integer);
CREATE TABLE public.lease (id uuid PRIMARY KEY, unit_id uuid NOT NULL, tenant_name text NOT NULL, start_date date NOT NULL, end_date date NOT NULL, rent_cents bigint NOT NULL, deposit_cents bigint NOT NULL, status text NOT NULL);
CREATE TABLE public.rent_ledger (id uuid PRIMARY KEY, lease_id uuid NOT NULL, period_start date NOT NULL, period_end date NOT NULL, amount_cents bigint NOT NULL, paid_at timestamptz, method text);
CREATE INDEX lease_unit_idx ON public.lease (unit_id, start_date DESC);
CREATE INDEX rent_ledger_lease_idx ON public.rent_ledger (lease_id, period_start DESC);
