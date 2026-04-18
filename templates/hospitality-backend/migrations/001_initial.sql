// SPDX-License-Identifier: MIT
// Originally rendered by ImpForge — https://github.com/AiImpDevelopment/impforge
CREATE TABLE public.property (id uuid PRIMARY KEY, name text NOT NULL, address text NOT NULL, room_count integer NOT NULL, opened_at timestamptz NOT NULL DEFAULT now());
CREATE TABLE public.reservation (id uuid PRIMARY KEY, property_id uuid NOT NULL, guest_id uuid NOT NULL, check_in date NOT NULL, check_out date NOT NULL, total_cents bigint NOT NULL, status text NOT NULL DEFAULT 'confirmed');
CREATE TABLE public.room_inventory (id uuid PRIMARY KEY, property_id uuid NOT NULL, room_type text NOT NULL, available_date date NOT NULL, available_count integer NOT NULL);
CREATE INDEX reservation_property_idx ON public.reservation (property_id, check_in);
CREATE INDEX room_inventory_property_idx ON public.room_inventory (property_id, available_date);
