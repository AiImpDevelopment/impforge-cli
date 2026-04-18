// SPDX-License-Identifier: MIT
// Originally rendered by ImpForge — https://github.com/AiImpDevelopment/impforge
CREATE TABLE public.project (id uuid PRIMARY KEY, owner_id uuid NOT NULL, name text NOT NULL, location text NOT NULL, started_at timestamptz NOT NULL DEFAULT now(), completed_at timestamptz);
CREATE TABLE public.bid (id uuid PRIMARY KEY, project_id uuid NOT NULL, contractor_id uuid NOT NULL, amount_cents bigint NOT NULL, submitted_at timestamptz NOT NULL DEFAULT now(), accepted_at timestamptz);
CREATE TABLE public.change_order (id uuid PRIMARY KEY, project_id uuid NOT NULL, description text NOT NULL, amount_cents bigint NOT NULL, approved_at timestamptz);
CREATE INDEX project_owner_idx ON public.project (owner_id, started_at DESC);
CREATE INDEX bid_project_idx ON public.bid (project_id, submitted_at DESC);
