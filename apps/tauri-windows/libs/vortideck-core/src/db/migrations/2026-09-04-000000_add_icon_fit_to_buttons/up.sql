ALTER TABLE buttons ADD COLUMN icon_fit TEXT NOT NULL DEFAULT 'cover'
  CHECK (icon_fit IN ('cover', 'contain'));
