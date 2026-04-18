// SPDX-License-Identifier: MIT
// Originally rendered by ImpForge — https://github.com/AiImpDevelopment/impforge
CREATE TABLE public.project (id uuid PRIMARY KEY, client_id uuid NOT NULL, name text NOT NULL, started_at timestamptz NOT NULL DEFAULT now(), completed_at timestamptz);
CREATE TABLE public.time_entry (id uuid PRIMARY KEY, project_id uuid NOT NULL, consultant_id uuid NOT NULL, hours numeric(6,2) NOT NULL, occurred_on date NOT NULL);
CREATE TABLE public.invoice (id uuid PRIMARY KEY, client_id uuid NOT NULL, total_cents bigint NOT NULL, issued_at timestamptz NOT NULL DEFAULT now(), paid_at timestamptz);
CREATE INDEX project_client_idx ON public.project (client_id, started_at DESC);
CREATE INDEX time_entry_project_idx ON public.time_entry (project_id, occurred_on DESC);
