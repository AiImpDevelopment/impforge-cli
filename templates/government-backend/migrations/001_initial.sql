// SPDX-License-Identifier: MIT
// Originally rendered by ImpForge — https://github.com/AiImpDevelopment/impforge
CREATE TABLE public.program (id uuid PRIMARY KEY, agency_code text NOT NULL, cfda text NOT NULL, name text NOT NULL, fiscal_year integer NOT NULL, obligation_cents bigint NOT NULL DEFAULT 0);
CREATE TABLE public.grant_award (id uuid PRIMARY KEY, program_id uuid NOT NULL, recipient_uei text NOT NULL, award_amount_cents bigint NOT NULL, period_start date NOT NULL, period_end date NOT NULL, status text NOT NULL);
CREATE TABLE public.case_record (id uuid PRIMARY KEY, case_number text NOT NULL UNIQUE, program_id uuid NOT NULL, opened_at timestamptz NOT NULL DEFAULT now(), closed_at timestamptz, caseworker text NOT NULL, status text NOT NULL);
CREATE TABLE public.public_record (id uuid PRIMARY KEY, foia_request_id text, case_id uuid, exemption_codes text[] NOT NULL DEFAULT '{}', created_at timestamptz NOT NULL DEFAULT now(), released_at timestamptz);
CREATE INDEX grant_award_program_idx ON public.grant_award (program_id, period_start DESC);
CREATE INDEX case_record_program_idx ON public.case_record (program_id, opened_at DESC);
