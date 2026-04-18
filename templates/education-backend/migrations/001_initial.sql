// SPDX-License-Identifier: MIT
// Originally rendered by ImpForge — https://github.com/AiImpDevelopment/impforge
CREATE TABLE public.enrollment (id uuid PRIMARY KEY, student_id uuid NOT NULL, course_id uuid NOT NULL, term text NOT NULL, enrolled_at timestamptz NOT NULL DEFAULT now(), withdrawn_at timestamptz);
CREATE TABLE public.grade_record (id uuid PRIMARY KEY, student_id uuid NOT NULL, course_id uuid NOT NULL, grade text NOT NULL, posted_at timestamptz NOT NULL DEFAULT now(), posted_by uuid NOT NULL);
CREATE TABLE public.transcript_lock (id uuid PRIMARY KEY, student_id uuid NOT NULL, locked_at timestamptz NOT NULL DEFAULT now(), reason text NOT NULL);
CREATE INDEX enrollment_student_idx ON public.enrollment (student_id, term);
CREATE INDEX grade_record_student_idx ON public.grade_record (student_id, course_id);
