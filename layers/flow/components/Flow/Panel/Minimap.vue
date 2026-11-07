<template lang="pug">
div(class="flow-minimap" :class="isCollapsed ? 'flow-minimap--collapsed' : ''")
  div(class="flow-minimap-header")
    div(class="flow-minimap-header-meta")
      span(class="flow-minimap-title") Minimap
      span(v-if="!isCollapsed" class="flow-minimap-zoom") {{ Math.round(camera.scale * 100) }}%
    button(
      class="flow-minimap-toggle"
      :title="isCollapsed ? 'Expand minimap' : 'Collapse minimap'"
      @click.stop="toggleCollapsed"
    ) {{ isCollapsed ? "+" : "-" }}

  div(
    v-show="!isCollapsed"
    ref="mapRef"
    class="flow-minimap-canvas"
    @mousedown.stop.prevent="handleMouseDown"
    title="Click or drag to pan canvas"
  )
    svg(
      :width="MAP_WIDTH"
      :height="MAP_HEIGHT"
      viewBox="0 0 220 140"
      class="flow-minimap-svg"
    )
      g
        path(
          v-for="edge in minimapEdges"
          :key="edge.id"
          :d="edge.d"
          fill="none"
          :stroke="edge.color"
          stroke-width="1.05"
          stroke-linecap="round"
          stroke-linejoin="round"
          opacity="0.62"
        )

      g
        rect(
          v-for="node in minimapNodes"
          :key="node.id"
          :x="node.x"
          :y="node.y"
          :width="node.w"
          :height="node.h"
          rx="1.5"
          :fill="node.fill"
          stroke="rgba(182, 205, 241, 0.34)"
          stroke-width="0.5"
        )

      rect(
        :x="viewportRect.x"
        :y="viewportRect.y"
        :width="viewportRect.w"
        :height="viewportRect.h"
        rx="2"
        class="flow-minimap-viewport"
      )
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from "vue";

interface MinimapNode {
  id: number;
  x: number;
  y: number;
  width?: number;
  height?: number;
  minimized?: boolean;
  invocationMode?: boolean;
  type?: string;
}

interface MinimapConnection {
  id: number;
  from: number;
  to: number;
  fromType?: string;
}

interface Camera {
  x: number;
  y: number;
  scale: number;
}

interface ViewportSize {
  width: number;
  height: number;
}

const props = withDefaults(
  defineProps<{
    nodes: MinimapNode[];
    connections: MinimapConnection[];
    camera: Camera;
    viewport: ViewportSize;
  }>(),
  {
    nodes: () => [],
    connections: () => [],
    viewport: () => ({ width: 0, height: 0 }),
  }
);

const emit = defineEmits<{
  panTo: [payload: { worldX: number; worldY: number }];
}>();

const MAP_WIDTH = 220;
const MAP_HEIGHT = 140;
const MAP_PADDING = 8;

const mapRef = ref<HTMLElement | null>(null);
const isDragging = ref(false);
const isCollapsed = ref(false);

const toggleCollapsed = () => {
  isCollapsed.value = !isCollapsed.value;
};

const getNodeWidth = (node: MinimapNode) => {
  if (typeof node.width === "number" && node.width > 0) return node.width;
  if (node.minimized) return 220;
  if (node.invocationMode) return 180;
  return 260;
};

const getNodeHeight = (node: MinimapNode) => {
  if (typeof node.height === "number" && node.height > 0) return node.height;
  if (node.minimized) return 56;
  if (node.invocationMode) return 120;
  return 180;
};

const worldViewport = computed(() => {
  const safeScale = Math.max(0.0001, props.camera.scale || 1);
  const width = Math.max(1, props.viewport.width || 1);
  const height = Math.max(1, props.viewport.height || 1);
  const x = -props.camera.x / safeScale;
  const y = -props.camera.y / safeScale;
  return { x, y, w: width / safeScale, h: height / safeScale };
});

const worldBounds = computed(() => {
  const viewport = worldViewport.value;

  if (props.nodes.length === 0) {
    const margin = 240;
    return {
      minX: viewport.x - margin,
      minY: viewport.y - margin,
      maxX: viewport.x + viewport.w + margin,
      maxY: viewport.y + viewport.h + margin,
    };
  }

  let minX = Number.POSITIVE_INFINITY;
  let minY = Number.POSITIVE_INFINITY;
  let maxX = Number.NEGATIVE_INFINITY;
  let maxY = Number.NEGATIVE_INFINITY;

  for (const node of props.nodes) {
    const w = getNodeWidth(node);
    const h = getNodeHeight(node);
    minX = Math.min(minX, node.x);
    minY = Math.min(minY, node.y);
    maxX = Math.max(maxX, node.x + w);
    maxY = Math.max(maxY, node.y + h);
  }

  minX = Math.min(minX, viewport.x);
  minY = Math.min(minY, viewport.y);
  maxX = Math.max(maxX, viewport.x + viewport.w);
  maxY = Math.max(maxY, viewport.y + viewport.h);

  const margin = 90;
  return {
    minX: minX - margin,
    minY: minY - margin,
    maxX: maxX + margin,
    maxY: maxY + margin,
  };
});

const worldToMapTransform = computed(() => {
  const bounds = worldBounds.value;
  const worldW = Math.max(1, bounds.maxX - bounds.minX);
  const worldH = Math.max(1, bounds.maxY - bounds.minY);
  const innerW = MAP_WIDTH - MAP_PADDING * 2;
  const innerH = MAP_HEIGHT - MAP_PADDING * 2;
  const scale = Math.max(0.0001, Math.min(innerW / worldW, innerH / worldH));
  const contentW = worldW * scale;
  const contentH = worldH * scale;
  const offsetX = (MAP_WIDTH - contentW) / 2;
  const offsetY = (MAP_HEIGHT - contentH) / 2;
  return { scale, offsetX, offsetY, ...bounds };
});

const toMap = (worldX: number, worldY: number) => {
  const t = worldToMapTransform.value;
  return {
    x: t.offsetX + (worldX - t.minX) * t.scale,
    y: t.offsetY + (worldY - t.minY) * t.scale,
  };
};

const toWorld = (mapX: number, mapY: number) => {
  const t = worldToMapTransform.value;
  return {
    x: t.minX + (mapX - t.offsetX) / t.scale,
    y: t.minY + (mapY - t.offsetY) / t.scale,
  };
};

const minimapNodes = computed(() => {
  return props.nodes.map((node) => {
    const topLeft = toMap(node.x, node.y);
    const w = Math.max(2.5, getNodeWidth(node) * worldToMapTransform.value.scale);
    const h = Math.max(2.5, getNodeHeight(node) * worldToMapTransform.value.scale);
    return {
      id: node.id,
      x: topLeft.x,
      y: topLeft.y,
      w,
      h,
      fill: node.minimized ? "rgba(79, 196, 255, 0.22)" : "rgba(171, 138, 255, 0.16)",
    };
  });
});

const nodeById = computed(() => new Map(props.nodes.map((node) => [node.id, node])));

const minimapEdges = computed(() => {
  const edges: Array<{ id: number; d: string; color: string }> = [];

  for (const conn of props.connections) {
    const source = nodeById.value.get(conn.from);
    const target = nodeById.value.get(conn.to);
    if (!source || !target) continue;

    const sourceW = getNodeWidth(source);
    const sourceH = getNodeHeight(source);
    const targetW = getNodeWidth(target);
    const targetH = getNodeHeight(target);

    const fromPoint = toMap(source.x + sourceW / 2, source.y + sourceH / 2);
    const toPoint = toMap(target.x + targetW / 2, target.y + targetH / 2);
    const dx = Math.max(4, Math.abs(toPoint.x - fromPoint.x) * 0.34);
    const d = `M ${fromPoint.x} ${fromPoint.y} C ${fromPoint.x + dx} ${fromPoint.y}, ${toPoint.x - dx} ${toPoint.y}, ${toPoint.x} ${toPoint.y}`;
    edges.push({
      id: conn.id,
      d,
      color:
        conn.fromType?.startsWith("flow-output")
          ? "rgba(219, 123, 255, 0.66)"
          : "rgba(128, 244, 175, 0.58)",
    });
  }

  return edges;
});

const viewportRect = computed(() => {
  const viewport = worldViewport.value;
  const topLeft = toMap(viewport.x, viewport.y);
  const bottomRight = toMap(viewport.x + viewport.w, viewport.y + viewport.h);
  const x = Math.max(0, Math.min(topLeft.x, MAP_WIDTH));
  const y = Math.max(0, Math.min(topLeft.y, MAP_HEIGHT));
  const w = Math.max(10, Math.min(MAP_WIDTH - x, bottomRight.x - topLeft.x));
  const h = Math.max(8, Math.min(MAP_HEIGHT - y, bottomRight.y - topLeft.y));
  return { x, y, w, h };
});

const getMapCoordinates = (event: MouseEvent) => {
  if (!mapRef.value) return null;
  const rect = mapRef.value.getBoundingClientRect();
  const localX = event.clientX - rect.left;
  const localY = event.clientY - rect.top;
  const clampedX = Math.max(0, Math.min(MAP_WIDTH, localX));
  const clampedY = Math.max(0, Math.min(MAP_HEIGHT, localY));
  return { x: clampedX, y: clampedY };
};

const panToPointer = (event: MouseEvent) => {
  const point = getMapCoordinates(event);
  if (!point) return;
  const world = toWorld(point.x, point.y);
  emit("panTo", { worldX: world.x, worldY: world.y });
};

const handleMouseMove = (event: MouseEvent) => {
  if (!isDragging.value) return;
  panToPointer(event);
};

const handleMouseUp = () => {
  if (!isDragging.value) return;
  isDragging.value = false;
  window.removeEventListener("mousemove", handleMouseMove);
  window.removeEventListener("mouseup", handleMouseUp);
};

const handleMouseDown = (event: MouseEvent) => {
  isDragging.value = true;
  panToPointer(event);
  window.addEventListener("mousemove", handleMouseMove);
  window.addEventListener("mouseup", handleMouseUp);
};

onBeforeUnmount(() => {
  window.removeEventListener("mousemove", handleMouseMove);
  window.removeEventListener("mouseup", handleMouseUp);
});
</script>

<style lang="scss" scoped>
.flow-minimap {
  width: 220px;
  border: 1px solid hsl(var(--border));
  border-radius: 0.85rem;
  background: hsl(var(--background2) / 0.88);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  box-shadow: 0 16px 32px -16px rgba(0, 0, 0, 0.4), inset 0 1px 0 rgba(255, 255, 255, 0.06);
  overflow: hidden;
  pointer-events: auto;
  transition: all 0.18s ease;
}

.flow-minimap--collapsed {
  width: 140px;
}

.flow-minimap-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.35rem;
  min-height: 2rem;
  padding: 0.35rem 0.55rem;
  background: hsl(var(--background2));
  border-bottom: 1px solid hsl(var(--border) / 0.4);
}

.flow-minimap-header-meta {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  min-width: 0;
}

.flow-minimap-title {
  font-size: 0.62rem;
  text-transform: uppercase;
  letter-spacing: 0.1em;
  color: hsl(var(--text3));
  font-weight: 700;
}

.flow-minimap-zoom {
  font-size: 0.62rem;
  color: hsl(var(--text2));
  border: 1px solid hsl(var(--border));
  border-radius: 999px;
  background: hsl(var(--background3));
  padding: 0.05rem 0.35rem;
  line-height: 1.25;
}

.flow-minimap-toggle {
  width: 1.25rem;
  height: 1.25rem;
  border-radius: 0.35rem;
  border: 1px solid hsl(var(--border));
  background: hsl(var(--background));
  color: hsl(var(--text2));
  font-size: 0.82rem;
  font-weight: 700;
  line-height: 1;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s ease;
  cursor: pointer;
}

.flow-minimap-toggle:hover {
  background: hsl(var(--background3));
  border-color: hsl(var(--border2));
  color: hsl(var(--text));
}

.flow-minimap-canvas {
  width: 220px;
  height: 140px;
  cursor: pointer;
  user-select: none;
  background: hsl(var(--background));
}

.flow-minimap-svg {
  display: block;
  width: 220px;
  height: 140px;
}

.flow-minimap-viewport {
  fill: hsl(var(--primary) / 0.1);
  stroke: hsl(var(--primary));
  stroke-width: 1.2px;
}
</style>
