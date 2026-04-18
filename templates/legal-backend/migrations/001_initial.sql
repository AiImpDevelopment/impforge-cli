// SPDX-License-Identifier: MIT
// Originally rendered by ImpForge — https://github.com/AiImpDevelopment/impforge
CREATE TABLE public.matter (id uuid PRIMARY KEY, client_id uuid NOT NULL, title text NOT NULL, opened_at timestamptz NOT NULL DEFAULT now(), closed_at timestamptz);
CREATE TABLE public.conflict_check (id uuid PRIMARY KEY, party_name text NOT NULL, related_matter_id uuid, checked_at timestamptz NOT NULL DEFAULT now(), result text NOT NULL);
CREATE TABLE public.trust_account_ledger (id uuid PRIMARY KEY, matter_id uuid NOT NULL, amount_cents bigint NOT NULL, occurred_at timestamptz NOT NULL DEFAULT now());
CREATE INDEX matter_client_idx ON public.matter (client_id, opened_at DESC);
CREATE INDEX trust_account_matter_idx ON public.trust_account_ledger (matter_id, occurred_at DESC);
