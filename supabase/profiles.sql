-- supabase/profiles.sql
-- Run this in the Supabase SQL editor (Dashboard → SQL Editor → New query)

-- ── 1. Profiles table ──────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS public.profiles (
    id               uuid PRIMARY KEY REFERENCES auth.users(id) ON DELETE CASCADE,
    scan_count       integer      NOT NULL DEFAULT 0,
    scan_count_month integer      NOT NULL DEFAULT 0,
    scan_reset_date  date         NOT NULL DEFAULT CURRENT_DATE,
    plan             text         NOT NULL DEFAULT 'free',
    stripe_customer_id text,
    created_at       timestamptz  NOT NULL DEFAULT now(),
    updated_at       timestamptz  NOT NULL DEFAULT now()
);

-- ── 2. Row Level Security ──────────────────────────────────────────────────
ALTER TABLE public.profiles ENABLE ROW LEVEL SECURITY;

-- Users can read their own profile
CREATE POLICY "profiles: select own"
    ON public.profiles FOR SELECT
    USING (auth.uid() = id);

-- Users can create their own profile (on first sign-in)
CREATE POLICY "profiles: insert own"
    ON public.profiles FOR INSERT
    WITH CHECK (auth.uid() = id);

-- Users can update their own profile
CREATE POLICY "profiles: update own"
    ON public.profiles FOR UPDATE
    USING (auth.uid() = id);

-- ── 3. increment_scan_quota RPC ────────────────────────────────────────────
-- Called server-side (service_role key) to atomically enforce monthly quotas.
-- Returns the new scan_count_month, or NULL if the quota is already exhausted.
CREATE OR REPLACE FUNCTION public.increment_scan_quota(p_user_id uuid)
RETURNS integer
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
AS $$
DECLARE
    v_plan        text;
    v_current     integer;
    v_quota       integer;
    v_new_count   integer;
BEGIN
    SELECT plan, scan_count_month
    INTO   v_plan, v_current
    FROM   public.profiles
    WHERE  id = p_user_id;

    IF NOT FOUND THEN
        -- Auto-create profile for users who signed up before this migration
        INSERT INTO public.profiles (id, scan_count, scan_count_month, plan, scan_reset_date)
        VALUES (p_user_id, 0, 0, 'free', CURRENT_DATE);
        v_plan    := 'free';
        v_current := 0;
    END IF;

    v_quota := CASE v_plan
        WHEN 'pro'     THEN 2147483647  -- unlimited
        WHEN 'starter' THEN 50
        ELSE                5           -- free
    END;

    IF v_current >= v_quota THEN
        RETURN NULL;  -- quota exhausted; caller returns 402
    END IF;

    UPDATE public.profiles
    SET    scan_count_month = scan_count_month + 1,
           scan_count       = scan_count + 1,
           updated_at       = now()
    WHERE  id = p_user_id
    RETURNING scan_count_month INTO v_new_count;

    RETURN v_new_count;
END;
$$;

-- Allow the service_role to call this function
GRANT EXECUTE ON FUNCTION public.increment_scan_quota(uuid) TO service_role;
