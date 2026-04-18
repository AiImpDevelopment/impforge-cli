// SPDX-License-Identifier: MIT
// Originally rendered by ImpForge — https://github.com/AiImpDevelopment/impforge
CREATE TABLE public.study (id uuid PRIMARY KEY, protocol text NOT NULL UNIQUE, phase text NOT NULL, sponsor text NOT NULL, start_date date NOT NULL, target_enrollment integer NOT NULL);
CREATE TABLE public.subject (id uuid PRIMARY KEY, study_id uuid NOT NULL, subject_code text NOT NULL, enrolled_at timestamptz NOT NULL DEFAULT now(), site_id uuid NOT NULL, status text NOT NULL);
CREATE TABLE public.adverse_event (id uuid PRIMARY KEY, subject_id uuid NOT NULL, onset_at timestamptz NOT NULL, meddra_llt text NOT NULL, severity text NOT NULL, serious boolean NOT NULL, reported_at timestamptz NOT NULL DEFAULT now());
CREATE TABLE public.audit_trail (id uuid PRIMARY KEY, actor text NOT NULL, table_name text NOT NULL, record_id uuid NOT NULL, action text NOT NULL, reason text, hash text NOT NULL, prior_hash text, occurred_at timestamptz NOT NULL DEFAULT now());
CREATE INDEX subject_study_idx ON public.subject (study_id, enrolled_at DESC);
CREATE INDEX adverse_event_subject_idx ON public.adverse_event (subject_id, onset_at DESC);
