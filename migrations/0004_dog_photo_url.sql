-- Add photo_url column to dogs table for dog profile pictures
ALTER TABLE dogs ADD COLUMN IF NOT EXISTS photo_url TEXT;
