-- Add execution_mode and execution_config columns to buttons table
ALTER TABLE buttons ADD COLUMN execution_mode TEXT DEFAULT 'run_once';
ALTER TABLE buttons ADD COLUMN execution_config TEXT DEFAULT NULL;

-- Migrate existing macro execution modes to buttons
UPDATE buttons 
SET execution_mode = (
    SELECT CASE 
        WHEN macros.macro_type = 'RunOnce' THEN 'run_once'
        WHEN macros.macro_type = 'RepeatWhilePressing' THEN 'repeat_while_pressing'
        WHEN macros.macro_type = 'Switch' THEN 'switch'
        WHEN macros.macro_type = 'Queue' THEN 'queue'
        ELSE 'run_once'
    END
    FROM macros 
    WHERE macros.id = CAST(REPLACE(REPLACE(buttons.action, '{"MacroId":', ''), '}', '') AS INTEGER)
    AND buttons.action LIKE '{"MacroId":%'
)
WHERE buttons.action LIKE '{"MacroId":%';
