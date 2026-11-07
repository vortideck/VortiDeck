-- Add extensible matching mode for focus window actions
ALTER TABLE focus_windows
ADD COLUMN match_mode TEXT NOT NULL DEFAULT 'smart';

-- Migrate existing process-only style records (empty title + process set)
UPDATE focus_windows
SET match_mode = 'process_only'
WHERE TRIM(COALESCE(window_title, '')) = ''
  AND TRIM(COALESCE(process_name, '')) != '';
