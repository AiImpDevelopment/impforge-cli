// SPDX-License-Identifier: MIT
// Originally rendered by ImpForge — https://github.com/AiImpDevelopment/impforge
CREATE TABLE public.aircraft (id uuid PRIMARY KEY, tail_number text NOT NULL UNIQUE, type_designator text NOT NULL, msn text NOT NULL, total_hours numeric(10,2) NOT NULL DEFAULT 0, total_cycles integer NOT NULL DEFAULT 0);
CREATE TABLE public.maintenance_log (id uuid PRIMARY KEY, aircraft_id uuid NOT NULL, work_card text NOT NULL, technician_id uuid NOT NULL, performed_at timestamptz NOT NULL DEFAULT now(), released_to_service_at timestamptz);
CREATE TABLE public.airworthiness_directive (id uuid PRIMARY KEY, ad_number text NOT NULL UNIQUE, applicable_aircraft text NOT NULL, effective_date date NOT NULL, compliance_due date NOT NULL);
CREATE INDEX maintenance_log_aircraft_idx ON public.maintenance_log (aircraft_id, performed_at DESC);
CREATE INDEX ad_compliance_idx ON public.airworthiness_directive (compliance_due);
