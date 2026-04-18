// SPDX-License-Identifier: MIT
// Originally rendered by ImpForge — https://github.com/AiImpDevelopment/impforge
CREATE TABLE public.meter_reading (id uuid PRIMARY KEY, meter_id uuid NOT NULL, reading_kwh numeric(12,3) NOT NULL, recorded_at timestamptz NOT NULL DEFAULT now(), source text NOT NULL DEFAULT 'ami');
CREATE TABLE public.outage_event (id uuid PRIMARY KEY, substation_id uuid NOT NULL, started_at timestamptz NOT NULL DEFAULT now(), restored_at timestamptz, customers_affected integer NOT NULL DEFAULT 0, cause_code text);
CREATE TABLE public.tariff_schedule (id uuid PRIMARY KEY, tariff_code text NOT NULL UNIQUE, effective_at timestamptz NOT NULL DEFAULT now(), expires_at timestamptz, rate_per_kwh_cents bigint NOT NULL);
CREATE INDEX meter_reading_meter_idx ON public.meter_reading (meter_id, recorded_at DESC);
CREATE INDEX outage_event_substation_idx ON public.outage_event (substation_id, started_at DESC);
