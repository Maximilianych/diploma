-- migrations/003_archive.sql

ALTER TABLE tasks ADD COLUMN is_archived INTEGER NOT NULL DEFAULT 0;