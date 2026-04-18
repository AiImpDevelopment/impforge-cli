// SPDX-License-Identifier: MIT
// Originally rendered by ImpForge — https://github.com/AiImpDevelopment/impforge
CREATE TABLE public.shipment (id uuid PRIMARY KEY, origin text NOT NULL, destination text NOT NULL, picked_up_at timestamptz, delivered_at timestamptz, weight_kg numeric(10,2) NOT NULL, hazmat boolean NOT NULL DEFAULT false);
CREATE TABLE public.vehicle (id uuid PRIMARY KEY, vin text NOT NULL UNIQUE, type text NOT NULL, capacity_kg numeric(10,2) NOT NULL, current_driver_id uuid);
CREATE TABLE public.driver_log (id uuid PRIMARY KEY, driver_id uuid NOT NULL, vehicle_id uuid NOT NULL, status text NOT NULL, recorded_at timestamptz NOT NULL DEFAULT now(), location_lat numeric(9,6), location_lon numeric(9,6));
CREATE INDEX shipment_status_idx ON public.shipment (delivered_at, picked_up_at);
CREATE INDEX driver_log_driver_idx ON public.driver_log (driver_id, recorded_at DESC);
