// SPDX-License-Identifier: MIT
// Originally rendered by ImpForge — https://github.com/AiImpDevelopment/impforge
CREATE TABLE public.subscriber (id uuid PRIMARY KEY, msisdn text NOT NULL UNIQUE, imsi text NOT NULL, plan_id uuid NOT NULL, activated_at timestamptz NOT NULL DEFAULT now(), status text NOT NULL);
CREATE TABLE public.rating_plan (id uuid PRIMARY KEY, name text NOT NULL, monthly_cents bigint NOT NULL, voice_rate_cents bigint NOT NULL, data_rate_cents bigint NOT NULL, sms_rate_cents bigint NOT NULL);
CREATE TABLE public.cdr (id uuid PRIMARY KEY, subscriber_id uuid NOT NULL, call_type text NOT NULL, start_at timestamptz NOT NULL, duration_sec integer NOT NULL, bytes_in bigint NOT NULL DEFAULT 0, bytes_out bigint NOT NULL DEFAULT 0, rated_cents bigint NOT NULL DEFAULT 0);
CREATE INDEX cdr_subscriber_start_idx ON public.cdr (subscriber_id, start_at DESC);
CREATE INDEX subscriber_status_idx ON public.subscriber (status, activated_at DESC);
