// SPDX-License-Identifier: MIT
// Originally rendered by ImpForge — https://github.com/AiImpDevelopment/impforge
CREATE TABLE public.phi_access_log (id uuid PRIMARY KEY, accessor_id uuid NOT NULL, patient_id uuid NOT NULL, accessed_at timestamptz NOT NULL DEFAULT now(), purpose text NOT NULL);
CREATE TABLE public.consent_record (id uuid PRIMARY KEY, patient_id uuid NOT NULL, consent_type text NOT NULL, granted_at timestamptz NOT NULL DEFAULT now(), revoked_at timestamptz);
CREATE TABLE public.encryption_audit (id uuid PRIMARY KEY, resource_id uuid NOT NULL, key_version integer NOT NULL, rotated_at timestamptz NOT NULL DEFAULT now());
CREATE INDEX phi_access_log_patient_idx ON public.phi_access_log (patient_id, accessed_at DESC);
CREATE INDEX consent_record_patient_idx ON public.consent_record (patient_id, consent_type);
