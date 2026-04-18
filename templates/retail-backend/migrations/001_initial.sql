// SPDX-License-Identifier: MIT
// Originally rendered by ImpForge — https://github.com/AiImpDevelopment/impforge
CREATE TABLE public.product (id uuid PRIMARY KEY, sku text NOT NULL UNIQUE, name text NOT NULL, price_cents bigint NOT NULL, created_at timestamptz NOT NULL DEFAULT now());
CREATE TABLE public.inventory_lot (id uuid PRIMARY KEY, product_id uuid NOT NULL, location_id uuid NOT NULL, quantity integer NOT NULL, received_at timestamptz NOT NULL DEFAULT now());
CREATE TABLE public.order_line (id uuid PRIMARY KEY, order_id uuid NOT NULL, product_id uuid NOT NULL, quantity integer NOT NULL, line_total_cents bigint NOT NULL);
CREATE INDEX product_sku_idx ON public.product (sku);
CREATE INDEX inventory_lot_product_idx ON public.inventory_lot (product_id, location_id);
