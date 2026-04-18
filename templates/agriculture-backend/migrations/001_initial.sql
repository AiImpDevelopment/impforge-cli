// SPDX-License-Identifier: MIT
// Originally rendered by ImpForge — https://github.com/AiImpDevelopment/impforge
CREATE TABLE public.field (id uuid PRIMARY KEY, farm_id uuid NOT NULL, name text NOT NULL, hectares numeric(10,2) NOT NULL, soil_type text);
CREATE TABLE public.crop_lot (id uuid PRIMARY KEY, field_id uuid NOT NULL, crop_type text NOT NULL, planted_at timestamptz NOT NULL DEFAULT now(), harvested_at timestamptz, lot_code text NOT NULL UNIQUE);
CREATE TABLE public.pesticide_application (id uuid PRIMARY KEY, field_id uuid NOT NULL, product_epa_reg text NOT NULL, applied_at timestamptz NOT NULL DEFAULT now(), rate_per_hectare numeric(10,4) NOT NULL, applicator_id uuid NOT NULL);
CREATE INDEX field_farm_idx ON public.field (farm_id, name);
CREATE INDEX crop_lot_field_idx ON public.crop_lot (field_id, planted_at DESC);
