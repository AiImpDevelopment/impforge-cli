// SPDX-License-Identifier: MIT
// Originally rendered by ImpForge — https://github.com/AiImpDevelopment/impforge
CREATE TABLE public.alert_event (id uuid PRIMARY KEY, source text NOT NULL, severity text NOT NULL, mitre_attack_id text, raw_event jsonb, occurred_at timestamptz NOT NULL DEFAULT now());
CREATE TABLE public.threat_intel (id uuid PRIMARY KEY, ioc_value text NOT NULL, ioc_type text NOT NULL, source text NOT NULL, confidence integer NOT NULL, first_seen timestamptz NOT NULL DEFAULT now());
CREATE TABLE public.incident_case (id uuid PRIMARY KEY, title text NOT NULL, severity text NOT NULL, opened_at timestamptz NOT NULL DEFAULT now(), closed_at timestamptz, lead_analyst_id uuid);
CREATE INDEX alert_event_severity_idx ON public.alert_event (severity, occurred_at DESC);
CREATE INDEX threat_intel_ioc_idx ON public.threat_intel (ioc_type, ioc_value);
