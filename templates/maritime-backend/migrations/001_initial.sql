// SPDX-License-Identifier: MIT
// Originally rendered by ImpForge — https://github.com/AiImpDevelopment/impforge
CREATE TABLE public.vessel (id uuid PRIMARY KEY, imo text NOT NULL UNIQUE, name text NOT NULL, flag_iso text NOT NULL, gt bigint NOT NULL, dwt bigint NOT NULL, vessel_type text NOT NULL);
CREATE TABLE public.voyage (id uuid PRIMARY KEY, vessel_id uuid NOT NULL, voyage_no text NOT NULL, departure_port text NOT NULL, arrival_port text NOT NULL, etd timestamptz NOT NULL, eta timestamptz NOT NULL, status text NOT NULL);
CREATE TABLE public.port_call (id uuid PRIMARY KEY, voyage_id uuid NOT NULL, port_locode text NOT NULL, arrival timestamptz NOT NULL, departure timestamptz, berth text, pilotage boolean NOT NULL DEFAULT false);
CREATE TABLE public.cargo (id uuid PRIMARY KEY, voyage_id uuid NOT NULL, bl_number text NOT NULL, commodity text NOT NULL, weight_mt numeric(12,3) NOT NULL, imdg_class text, shipper text NOT NULL, consignee text NOT NULL);
CREATE INDEX voyage_vessel_idx ON public.voyage (vessel_id, etd DESC);
CREATE INDEX port_call_voyage_idx ON public.port_call (voyage_id, arrival);
