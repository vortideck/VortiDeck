/**
 * Type color mapping for flow node ports
 * Inspired by ComfyUI's visual type system
 *
 * Each data type gets a distinct color for better visual clarity
 * when connecting nodes together.
 */

export const TYPE_COLORS: Record<string, string> = {
  // Primitive types
  string: '#4CAF50',    // Green
  number: '#2196F3',    // Blue
  boolean: '#FF9800',   // Orange

  // Complex types
  array: '#9C27B0',     // Purple
  object: '#607D8B',    // Grey

  // Media types
  audio: '#F59E0B',     // Amber
  image: '#8B5CF6',     // Violet
  video: '#EC4899',     // Pink

  // Data types
  text: '#10B981',      // Emerald (similar to string but lighter)
  json: '#6366F1',      // Indigo

  // Special types
  any: '#94A3B8',       // Slate (neutral)
  flow: '#EF4444',      // Red (for flow control)

  // Default fallback
  default: '#64748B',   // Slate-500
};

/**
 * Get color for a given type string
 *
 * @param type - The data type (e.g., 'string', 'number', 'audio')
 * @returns Hex color string
 *
 * @example
 * getTypeColor('string') // returns '#4CAF50'
 * getTypeColor('unknown') // returns '#64748B' (default)
 */
export function getTypeColor(type: string | undefined | null): string {
  if (!type || typeof type !== 'string') return TYPE_COLORS.default;

  // Normalize type string (lowercase, trim)
  const normalizedType = type.toLowerCase().trim();

  // Return color or default
  return TYPE_COLORS[normalizedType] || TYPE_COLORS.default;
}

/**
 * Get color with alpha transparency
 *
 * @param type - The data type
 * @param alpha - Alpha value (0-1)
 * @returns RGBA color string
 *
 * @example
 * getTypeColorWithAlpha('string', 0.5) // returns 'rgba(76, 175, 80, 0.5)'
 */
export function getTypeColorWithAlpha(type: string | undefined | null, alpha: number = 1): string {
  const hex = getTypeColor(type);

  // Convert hex to RGB
  const r = parseInt(hex.slice(1, 3), 16);
  const g = parseInt(hex.slice(3, 5), 16);
  const b = parseInt(hex.slice(5, 7), 16);

  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}
