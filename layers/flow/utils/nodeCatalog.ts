export interface CatalogNode {
  name: string;
  label: string;
  category: string;
  category_display_name?: string | null;
  subcategory?: string | null;
}

export interface NodeCatalogGroup<T extends CatalogNode = CatalogNode> {
  id: string;
  label: string;
  items: T[];
}

export interface NodeCatalogCategory<T extends CatalogNode = CatalogNode> {
  id: string;
  label: string;
  icon: string;
  groups: NodeCatalogGroup<T>[];
  items: T[];
}

const CATEGORY_ALIASES: Record<string, string> = {
  CONTROL_FLOW: "CONTROL",
  UTILITY: "UTILITIES",
};

const CATEGORY_PRESENTATION: Record<string, { label: string; icon: string; order: number }> = {
  TRIGGER: { label: "Start Triggers", icon: "◉", order: 10 },
  CONTROL: { label: "Control Flow", icon: "◆", order: 20 },
  AI: { label: "AI Tools", icon: "✦", order: 30 },
  AUDIO: { label: "Audio Tools", icon: "♪", order: 40 },
  ACTION: { label: "Actions", icon: "▶", order: 50 },
  STREAM: { label: "Streaming Tools", icon: "●", order: 60 },
  INTEGRATION: { label: "Integrations", icon: "◇", order: 70 },
  SYSTEM: { label: "System Tools", icon: "■", order: 80 },
  VARIABLES: { label: "Variables", icon: "$", order: 90 },
  MEDIA: { label: "Media Tools", icon: "▣", order: 100 },
  UI: { label: "UI", icon: "□", order: 110 },
  WEBHOOK: { label: "Webhooks", icon: "↗", order: 120 },
  MACRO: { label: "Macros & Automation", icon: "⌘", order: 130 },
  UTILITIES: { label: "Utilities", icon: "•", order: 140 },
  PLUGINS: { label: "Plugins", icon: "⬡", order: 150 },
  DEVELOPMENT: { label: "Development", icon: "⚙", order: 1000 },
};

const titleCaseIdentifier = (value: string): string =>
  value
    .split("_")
    .filter(Boolean)
    .map((word) => word.charAt(0) + word.slice(1).toLowerCase())
    .join(" ");

export const canonicalNodeCategory = (category: unknown): string => {
  const normalized = String(category || "OTHER").trim().toUpperCase() || "OTHER";
  return CATEGORY_ALIASES[normalized] || normalized;
};

export const nodeCategoryLabel = (category: string, items: CatalogNode[] = []): string => {
  const labels = Array.from(
    new Set(
      items
        .map((item) => item.category_display_name?.trim())
        .filter((label): label is string => Boolean(label)),
    ),
  );
  return labels[0] || CATEGORY_PRESENTATION[category]?.label || titleCaseIdentifier(category);
};

export const buildNodeCatalog = <T extends CatalogNode>(items: T[]): NodeCatalogCategory<T>[] => {
  const categories = new Map<string, T[]>();
  for (const item of items) {
    if (!item || typeof item.name !== "string") continue;
    const category = canonicalNodeCategory(item.category);
    const normalizedItem = item.category === category ? item : { ...item, category };
    if (!categories.has(category)) categories.set(category, []);
    categories.get(category)!.push(normalizedItem as T);
  }

  return Array.from(categories.entries())
    .map(([id, categoryItems]) => {
      const groups = new Map<string, T[]>();
      for (const item of categoryItems) {
        const group = item.subcategory?.trim() || "General";
        if (!groups.has(group)) groups.set(group, []);
        groups.get(group)!.push(item);
      }
      return {
        id,
        label: nodeCategoryLabel(id, categoryItems),
        icon: CATEGORY_PRESENTATION[id]?.icon || "•",
        items: categoryItems,
        groups: Array.from(groups.entries())
          .map(([label, groupItems]) => ({ id: label, label, items: groupItems }))
          .sort((a, b) => {
            if (a.id === "General") return -1;
            if (b.id === "General") return 1;
            return a.label.localeCompare(b.label);
          }),
      };
    })
    .sort((a, b) => {
      const orderA = CATEGORY_PRESENTATION[a.id]?.order ?? 500;
      const orderB = CATEGORY_PRESENTATION[b.id]?.order ?? 500;
      return orderA - orderB || a.label.localeCompare(b.label);
    });
};

export const validateNodeCatalog = (items: CatalogNode[]): string[] => {
  const errors: string[] = [];
  const ids = new Set<string>();
  const labelsByCategory = new Map<string, Set<string>>();

  for (const item of items) {
    if (ids.has(item.name)) errors.push(`Duplicate node ID: ${item.name}`);
    ids.add(item.name);

    const category = canonicalNodeCategory(item.category);
    const display = item.category_display_name?.trim();
    if (display) {
      if (!labelsByCategory.has(category)) labelsByCategory.set(category, new Set());
      labelsByCategory.get(category)!.add(display);
    }
  }

  for (const [category, labels] of labelsByCategory) {
    if (labels.size > 1) {
      errors.push(`Category ${category} has conflicting labels: ${Array.from(labels).join(", ")}`);
    }
  }
  return errors;
};
