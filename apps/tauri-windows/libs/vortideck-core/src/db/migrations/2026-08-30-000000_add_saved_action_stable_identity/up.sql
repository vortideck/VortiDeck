CREATE TABLE saved_actions_v2 (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    stable_id TEXT NOT NULL UNIQUE,
    integration_id TEXT NOT NULL,
    name TEXT NOT NULL,
    action TEXT NOT NULL,
    params TEXT NOT NULL
);

INSERT INTO saved_actions_v2 (id, stable_id, integration_id, name, action, params)
SELECT
    id,
    lower(
        hex(randomblob(4)) || '-' ||
        hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 2) || '-' ||
        substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2) || '-' ||
        hex(randomblob(6))
    ),
    integration_id,
    name,
    action,
    params
FROM saved_actions;

-- Rewrite persisted execution references once. Runtime code only understands
-- SavedActionId after this migration.
UPDATE buttons
SET action = json_object(
    'SavedActionId',
    (SELECT stable_id FROM saved_actions_v2
     WHERE id = json_extract(buttons.action, '$.ObsActionId'))
)
WHERE action IS NOT NULL
  AND json_valid(action)
  AND json_type(action, '$.ObsActionId') = 'integer';

UPDATE webhooks
SET action = json_object(
    'SavedActionId',
    (SELECT stable_id FROM saved_actions_v2
     WHERE id = json_extract(webhooks.action, '$.ObsActionId'))
)
WHERE action IS NOT NULL
  AND json_valid(action)
  AND json_type(action, '$.ObsActionId') = 'integer';

UPDATE stream_sessions
SET commands = (
    SELECT json_group_array(json(
        CASE
            WHEN json_valid(json_extract(command.value, '$.action'))
             AND json_type(json_extract(command.value, '$.action'), '$.ObsActionId') = 'integer'
            THEN json_set(
                command.value,
                '$.action',
                printf(
                    '{"SavedActionId":"%s"}',
                    (SELECT stable_id FROM saved_actions_v2
                     WHERE id = json_extract(json_extract(command.value, '$.action'), '$.ObsActionId'))
                )
            )
            ELSE command.value
        END
    ))
    FROM json_each(stream_sessions.commands) AS command
)
WHERE json_valid(commands)
  AND json_type(commands) = 'array';

UPDATE flows
SET data = json_set(
    data,
    '$.nodes',
    json((
        SELECT json_group_array(json(
            CASE
                WHEN json_extract(node.value, '$.nodeType') = 'assignment-action'
                THEN json_remove(
                    json_set(
                        node.value,
                        '$.nodeType', 'invoke-action',
                        '$.inputs.actionReference', json(json_object(
                            'kind', CASE json_extract(node.value, '$.inputs.assignmentRef.actionType')
                                WHEN 'obs' THEN 'saved_action'
                                ELSE json_extract(node.value, '$.inputs.assignmentRef.actionType')
                            END,
                            'id', CASE json_extract(node.value, '$.inputs.assignmentRef.actionType')
                                WHEN 'obs' THEN (
                                    SELECT stable_id FROM saved_actions_v2
                                    WHERE id = json_extract(node.value, '$.inputs.assignmentRef.actionData')
                                )
                                ELSE CAST(json_extract(node.value, '$.inputs.assignmentRef.actionData') AS TEXT)
                            END
                        )),
                        '$.uiData.customProperties.actionReference', json(json_object(
                            'kind', CASE json_extract(node.value, '$.inputs.assignmentRef.actionType')
                                WHEN 'obs' THEN 'saved_action'
                                ELSE json_extract(node.value, '$.inputs.assignmentRef.actionType')
                            END,
                            'id', CASE json_extract(node.value, '$.inputs.assignmentRef.actionType')
                                WHEN 'obs' THEN (
                                    SELECT stable_id FROM saved_actions_v2
                                    WHERE id = json_extract(node.value, '$.inputs.assignmentRef.actionData')
                                )
                                ELSE CAST(json_extract(node.value, '$.inputs.assignmentRef.actionData') AS TEXT)
                            END
                        )),
                        '$.uiData.customProperties.invocationMode', json('true')
                    ),
                    '$.inputs.assignmentRef',
                    '$.uiData.customProperties.assignmentRef',
                    '$.uiData.customProperties.assignmentMode'
                )
                ELSE node.value
            END
        ))
        FROM json_each(flows.data, '$.nodes') AS node
    ))
)
WHERE json_valid(data)
  AND json_type(data, '$.nodes') = 'array'
  AND EXISTS (
      SELECT 1 FROM json_each(flows.data, '$.nodes') AS node
      WHERE json_extract(node.value, '$.nodeType') = 'assignment-action'
  );

DROP TABLE saved_actions;
ALTER TABLE saved_actions_v2 RENAME TO saved_actions;
CREATE UNIQUE INDEX saved_actions_stable_id_idx ON saved_actions(stable_id);
