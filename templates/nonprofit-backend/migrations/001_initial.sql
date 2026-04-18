// SPDX-License-Identifier: MIT
// Originally rendered by ImpForge — https://github.com/AiImpDevelopment/impforge
CREATE TABLE public.donor (id uuid PRIMARY KEY, name text NOT NULL, email text NOT NULL, type text NOT NULL DEFAULT 'individual', anonymized boolean NOT NULL DEFAULT false);
CREATE TABLE public.donation (id uuid PRIMARY KEY, donor_id uuid NOT NULL, amount_cents bigint NOT NULL, designation text, received_at timestamptz NOT NULL DEFAULT now(), acknowledged_at timestamptz);
CREATE TABLE public.grant_award (id uuid PRIMARY KEY, funder_id uuid NOT NULL, project_id uuid, amount_cents bigint NOT NULL, awarded_at timestamptz NOT NULL DEFAULT now(), reporting_deadline date);
CREATE INDEX donation_donor_idx ON public.donation (donor_id, received_at DESC);
CREATE INDEX grant_award_funder_idx ON public.grant_award (funder_id, awarded_at DESC);
