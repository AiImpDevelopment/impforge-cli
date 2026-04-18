// SPDX-License-Identifier: MIT
// Originally rendered by ImpForge — https://github.com/AiImpDevelopment/impforge
CREATE TABLE public.policy (id uuid PRIMARY KEY, policyholder_id uuid NOT NULL, policy_number text NOT NULL UNIQUE, effective_at timestamptz NOT NULL, expires_at timestamptz NOT NULL, premium_cents bigint NOT NULL);
CREATE TABLE public.claim (id uuid PRIMARY KEY, policy_id uuid NOT NULL, claimant_id uuid NOT NULL, reported_at timestamptz NOT NULL DEFAULT now(), status text NOT NULL, paid_cents bigint NOT NULL DEFAULT 0);
CREATE TABLE public.reserve_calc (id uuid PRIMARY KEY, policy_id uuid NOT NULL, reserved_cents bigint NOT NULL, method text NOT NULL, calculated_at timestamptz NOT NULL DEFAULT now());
CREATE INDEX policy_holder_idx ON public.policy (policyholder_id, effective_at DESC);
CREATE INDEX claim_policy_idx ON public.claim (policy_id, reported_at DESC);
