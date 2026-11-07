-- Your SQL goes here










ALTER TABLE `programs` ADD COLUMN `program_type` TEXT NOT NULL DEFAULT 'command';
ALTER TABLE `programs` ADD COLUMN `execution_mode` TEXT NOT NULL DEFAULT 'terminal';


