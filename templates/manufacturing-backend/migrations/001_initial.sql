// SPDX-License-Identifier: MIT
// Originally rendered by ImpForge — https://github.com/AiImpDevelopment/impforge
CREATE TABLE public.work_order (id uuid PRIMARY KEY, part_number text NOT NULL, quantity integer NOT NULL, scheduled_at timestamptz NOT NULL, completed_at timestamptz, status text NOT NULL DEFAULT 'open');
CREATE TABLE public.bom_revision (id uuid PRIMARY KEY, parent_part text NOT NULL, child_part text NOT NULL, quantity numeric(10,4) NOT NULL, revision integer NOT NULL, effective_at timestamptz NOT NULL DEFAULT now());
CREATE TABLE public.quality_inspection (id uuid PRIMARY KEY, work_order_id uuid NOT NULL, inspector_id uuid NOT NULL, result text NOT NULL, inspected_at timestamptz NOT NULL DEFAULT now(), notes text);
CREATE INDEX work_order_status_idx ON public.work_order (status, scheduled_at);
CREATE INDEX bom_revision_parent_idx ON public.bom_revision (parent_part, revision DESC);
