// SPDX-License-Identifier: MIT
// Originally rendered by ImpForge — https://github.com/AiImpDevelopment/impforge
CREATE TABLE public.menu_item (id uuid PRIMARY KEY, sku text NOT NULL UNIQUE, name text NOT NULL, allergens text[], price_cents bigint NOT NULL);
CREATE TABLE public.batch_lot (id uuid PRIMARY KEY, item_id uuid NOT NULL, lot_code text NOT NULL UNIQUE, produced_at timestamptz NOT NULL DEFAULT now(), expires_at timestamptz NOT NULL, supplier_id uuid);
CREATE TABLE public.haccp_check (id uuid PRIMARY KEY, batch_lot_id uuid NOT NULL, ccp text NOT NULL, measurement numeric(10,4) NOT NULL, passed boolean NOT NULL, checked_at timestamptz NOT NULL DEFAULT now());
CREATE INDEX menu_item_sku_idx ON public.menu_item (sku);
CREATE INDEX batch_lot_item_idx ON public.batch_lot (item_id, produced_at DESC);
