// SPDX-License-Identifier: MIT
// Originally rendered by ImpForge — https://github.com/AiImpDevelopment/impforge
CREATE TABLE public.site (id uuid PRIMARY KEY, name text NOT NULL, msha_id text NOT NULL UNIQUE, commodity text NOT NULL, country_iso text NOT NULL, lat numeric(9,6) NOT NULL, lon numeric(9,6) NOT NULL);
CREATE TABLE public.shift (id uuid PRIMARY KEY, site_id uuid NOT NULL, shift_start timestamptz NOT NULL, shift_end timestamptz NOT NULL, supervisor text NOT NULL, crew_size integer NOT NULL);
CREATE TABLE public.tailings_dam (id uuid PRIMARY KEY, site_id uuid NOT NULL, name text NOT NULL, height_m numeric(10,2) NOT NULL, capacity_m3 bigint NOT NULL, consequence_class text NOT NULL, last_inspection date NOT NULL);
CREATE TABLE public.incident (id uuid PRIMARY KEY, site_id uuid NOT NULL, occurred_at timestamptz NOT NULL, category text NOT NULL, severity text NOT NULL, msha_reportable boolean NOT NULL, summary text NOT NULL);
CREATE INDEX incident_site_idx ON public.incident (site_id, occurred_at DESC);
