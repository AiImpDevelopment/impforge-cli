// SPDX-License-Identifier: MIT
// Originally rendered by ImpForge — https://github.com/AiImpDevelopment/impforge
CREATE TABLE public.audit_log (id uuid PRIMARY KEY, actor_id uuid NOT NULL, action text NOT NULL, occurred_at timestamptz NOT NULL DEFAULT now(), payload jsonb);
CREATE TABLE public.ledger_entry (id uuid PRIMARY KEY, account_id uuid NOT NULL, amount_cents bigint NOT NULL, currency text NOT NULL, posted_at timestamptz NOT NULL DEFAULT now());
CREATE TABLE public.rate_limit (key text PRIMARY KEY, hits integer NOT NULL DEFAULT 0, window_start timestamptz NOT NULL);
CREATE INDEX audit_log_actor_idx ON public.audit_log (actor_id, occurred_at DESC);
CREATE INDEX ledger_entry_account_idx ON public.ledger_entry (account_id, posted_at DESC);
