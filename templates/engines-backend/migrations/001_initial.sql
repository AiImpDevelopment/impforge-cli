// SPDX-License-Identifier: MIT
// Originally rendered by ImpForge — https://github.com/AiImpDevelopment/impforge
CREATE TABLE public.engine_part (id uuid PRIMARY KEY, part_number text NOT NULL UNIQUE, description text NOT NULL, family_certification_number text, supersession_chain text[]);
CREATE TABLE public.qa_inspection (id uuid PRIMARY KEY, part_id uuid NOT NULL, inspector_id uuid NOT NULL, result text NOT NULL, inspected_at timestamptz NOT NULL DEFAULT now(), measurements jsonb);
CREATE TABLE public.emission_test (id uuid PRIMARY KEY, engine_serial text NOT NULL, test_cycle text NOT NULL, nox_g_kwh numeric(8,4), pm_g_kwh numeric(8,4), tested_at timestamptz NOT NULL DEFAULT now());
CREATE INDEX qa_inspection_part_idx ON public.qa_inspection (part_id, inspected_at DESC);
CREATE INDEX emission_test_serial_idx ON public.emission_test (engine_serial, tested_at DESC);
