// SPDX-License-Identifier: MIT
// Originally rendered by ImpForge — https://github.com/AiImpDevelopment/impforge
CREATE TABLE public.content_asset (id uuid PRIMARY KEY, kind text NOT NULL, title text NOT NULL, creator_id uuid NOT NULL, published_at timestamptz, license text NOT NULL DEFAULT 'all_rights_reserved');
CREATE TABLE public.ad_placement (id uuid PRIMARY KEY, asset_id uuid NOT NULL, advertiser_id uuid NOT NULL, slot text NOT NULL, served_at timestamptz NOT NULL DEFAULT now(), revenue_cents bigint NOT NULL DEFAULT 0);
CREATE TABLE public.royalty_split (id uuid PRIMARY KEY, asset_id uuid NOT NULL, beneficiary_id uuid NOT NULL, percent numeric(5,2) NOT NULL, effective_at timestamptz NOT NULL DEFAULT now());
CREATE INDEX content_asset_creator_idx ON public.content_asset (creator_id, published_at DESC);
CREATE INDEX ad_placement_asset_idx ON public.ad_placement (asset_id, served_at DESC);
