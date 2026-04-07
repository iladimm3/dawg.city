-- Add Stripe customer ID to users for billing portal and subscription management
ALTER TABLE users ADD COLUMN IF NOT EXISTS stripe_customer_id TEXT UNIQUE;
