import { invoke } from "@tauri-apps/api/core";
import type {
  DeckWidgetInstance,
  WidgetBinding,
  WidgetExecutionPolicy,
  WidgetPlacement,
  WidgetValueTransform,
} from "@base/types/deckWidget";

interface PersistedDeckItem {
  id: string;
  deck_id: number;
  definition_id: string;
  package_id: string | null;
  contribution_id: string | null;
  config_json: string;
  config_version: number;
  appearance_json: string;
  legacy_button_id: number | null;
  enabled: boolean;
}

interface PersistedDeckItemLayout {
  item_id: string;
  layout_key: string;
  x: number;
  y: number;
  col_span: number;
  row_span: number;
}

interface PersistedDeckItemBinding {
  id: string;
  item_id: string;
  port_id: string;
  action_json: string;
  policy_json: string;
  transform_json: string | null;
}

interface PersistedDeckItemDocument {
  item: PersistedDeckItem;
  layouts: PersistedDeckItemLayout[];
  bindings: PersistedDeckItemBinding[];
}

interface PersistedDeckWidgetDocument {
  deck_id: number;
  revision: number;
  updated_at: string;
  items: PersistedDeckItemDocument[];
}

export interface DeckWidgetDocumentSnapshot {
  deckId: number;
  revision: number;
  updatedAt: string;
  instances: DeckWidgetInstance[];
  legacyBacked: boolean;
}

const parseJson = <T>(value: string, fallback: T): T => {
  try {
    return JSON.parse(value) as T;
  } catch {
    return fallback;
  }
};

const fromPersisted = (document: PersistedDeckWidgetDocument): DeckWidgetDocumentSnapshot => ({
  deckId: document.deck_id,
  revision: document.revision,
  updatedAt: document.updated_at,
  legacyBacked: document.revision === 0 && document.items.length === 0,
  instances: document.items.map(({ item, layouts, bindings }) => ({
    id: item.id,
    definitionId: item.definition_id,
    packageId: item.package_id ?? undefined,
    contributionId: item.contribution_id ?? undefined,
    configVersion: item.config_version,
    config: parseJson<Record<string, unknown>>(item.config_json, {}),
    appearance: parseJson<Record<string, unknown>>(item.appearance_json, {}),
    layouts: Object.fromEntries(layouts.map((layout) => [layout.layout_key, {
      x: layout.x,
      y: layout.y,
      cols: layout.col_span,
      rows: layout.row_span,
    } satisfies WidgetPlacement])),
    bindings: bindings.map((binding): WidgetBinding => ({
      id: binding.id,
      portId: binding.port_id,
      action: parseJson<unknown>(binding.action_json, null),
      policy: parseJson<WidgetExecutionPolicy>(binding.policy_json, { type: "run_once" }),
      transform: binding.transform_json
        ? parseJson<WidgetValueTransform | undefined>(binding.transform_json, undefined)
        : undefined,
    })),
    legacyButtonId: item.legacy_button_id ?? undefined,
    enabled: item.enabled,
  })),
});

const toPersisted = (deckId: number, instance: DeckWidgetInstance): PersistedDeckItemDocument => ({
  item: {
    id: instance.id,
    deck_id: deckId,
    definition_id: instance.definitionId,
    package_id: instance.packageId ?? null,
    contribution_id: instance.contributionId ?? null,
    config_json: JSON.stringify(instance.config),
    config_version: instance.configVersion,
    appearance_json: JSON.stringify(instance.appearance ?? {}),
    legacy_button_id: instance.legacyButtonId ?? null,
    enabled: instance.enabled,
  },
  layouts: Object.entries(instance.layouts).map(([layoutKey, placement]) => ({
    item_id: instance.id,
    layout_key: layoutKey,
    x: placement.x,
    y: placement.y,
    col_span: placement.cols,
    row_span: placement.rows,
  })),
  bindings: instance.bindings.map((binding) => ({
    id: binding.id,
    item_id: instance.id,
    port_id: binding.portId,
    action_json: JSON.stringify(binding.action),
    policy_json: JSON.stringify(binding.policy),
    transform_json: binding.transform ? JSON.stringify(binding.transform) : null,
  })),
});

export const useDeckWidgetDocument = () => {
  const load = async (deckId: number): Promise<DeckWidgetDocumentSnapshot> => (
    fromPersisted(await invoke<PersistedDeckWidgetDocument>("get_deck_widget_document", { deckId }))
  );

  const save = async (
    deckId: number,
    expectedRevision: number,
    instances: DeckWidgetInstance[],
  ): Promise<DeckWidgetDocumentSnapshot> => fromPersisted(
    await invoke<PersistedDeckWidgetDocument>("save_deck_widget_document", {
      deckId,
      expectedRevision,
      items: instances.map((instance) => toPersisted(deckId, instance)),
    }),
  );

  return { load, save };
};

export const deckWidgetDocumentMapper = { fromPersisted, toPersisted };
