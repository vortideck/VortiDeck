-- Correct development databases that applied the original stable-identity
-- migration before its data rewrite was finalized. This is a one-time data
-- conversion; runtime code continues to accept only SavedActionId.
UPDATE buttons
SET action = json_object(
    'SavedActionId',
    (SELECT stable_id FROM saved_actions
     WHERE id = json_extract(buttons.action, '$.ObsActionId'))
)
WHERE action IS NOT NULL
  AND json_valid(action)
  AND json_type(action, '$.ObsActionId') = 'integer'
  AND EXISTS (
      SELECT 1 FROM saved_actions
      WHERE id = json_extract(buttons.action, '$.ObsActionId')
  );

-- A dangling numeric reference has no safe stable target. Treat the button as
-- unassigned instead of retaining an action that the canonical runtime rejects.
UPDATE buttons
SET action = NULL
WHERE action IS NOT NULL
  AND json_valid(action)
  AND json_type(action, '$.ObsActionId') = 'integer';

UPDATE deck_item_bindings
SET action_json = json_object(
    'SavedActionId',
    (SELECT stable_id FROM saved_actions
     WHERE id = json_extract(deck_item_bindings.action_json, '$.ObsActionId'))
)
WHERE json_valid(action_json)
  AND json_type(action_json, '$.ObsActionId') = 'integer'
  AND EXISTS (
      SELECT 1 FROM saved_actions
      WHERE id = json_extract(deck_item_bindings.action_json, '$.ObsActionId')
  );

UPDATE deck_item_bindings
SET action_json = 'null'
WHERE json_valid(action_json)
  AND json_type(action_json, '$.ObsActionId') = 'integer';

UPDATE webhooks
SET action = json_object(
    'SavedActionId',
    (SELECT stable_id FROM saved_actions
     WHERE id = json_extract(webhooks.action, '$.ObsActionId'))
)
WHERE action IS NOT NULL
  AND json_valid(action)
  AND json_type(action, '$.ObsActionId') = 'integer'
  AND EXISTS (
      SELECT 1 FROM saved_actions
      WHERE id = json_extract(webhooks.action, '$.ObsActionId')
  );

UPDATE webhooks
SET action = NULL
WHERE action IS NOT NULL
  AND json_valid(action)
  AND json_type(action, '$.ObsActionId') = 'integer';

UPDATE stream_sessions
SET commands = (
    SELECT json_group_array(json(
        CASE
            WHEN json_valid(json_extract(command.value, '$.action'))
             AND json_type(json_extract(command.value, '$.action'), '$.ObsActionId') = 'integer'
             AND EXISTS (
                 SELECT 1 FROM saved_actions
                 WHERE id = json_extract(json_extract(command.value, '$.action'), '$.ObsActionId')
             )
            THEN json_set(
                command.value,
                '$.action',
                printf(
                    '{"SavedActionId":"%s"}',
                    (SELECT stable_id FROM saved_actions
                     WHERE id = json_extract(json_extract(command.value, '$.action'), '$.ObsActionId'))
                )
            )
            WHEN json_valid(json_extract(command.value, '$.action'))
             AND json_type(json_extract(command.value, '$.action'), '$.ObsActionId') = 'integer'
            THEN json_set(command.value, '$.action', '', '$.enabled', json('false'))
            ELSE command.value
        END
    ))
    FROM json_each(stream_sessions.commands) AS command
)
WHERE json_valid(commands)
  AND json_type(commands) = 'array';

