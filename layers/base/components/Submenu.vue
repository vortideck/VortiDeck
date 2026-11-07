<template lang="pug">
div(class="deck-submenu flex flex-col gap-y-2 overflow-visible")
    div(class="submenu-header flex gap-x-2 items-center py-1.5 px-2 rounded-lg" @click="isOpen = !isOpen")
        slot(name="context")
        HugeiconsArrowDown01(:class="isOpen && items2.length > 0 ? 'rotate-180' : ''" class="ml-auto w-5 h-5 transition-transform duration-200" )

    transition(name="submenu" @enter="enter" @leave="leave")
        div(v-show="isOpen" class="submenu-content w-full" ref="submenuContent")
            p(v-if="items2.length === 0" class="submenu-empty") No items yet
            div(v-else class="submenu-list")
                div(
                  v-for="item in items2"
                  :key="item.id"
                  @click="openItem(item)"
                  @contextmenu.prevent="editable && handleContextMenu($event, item)"
                  draggable="true"
                  @dragstart="useHandleDragAction($event, getExecutionReferenceId(item), actionType, item.name)"
                  class="submenu-row group"
                )
                  p(class="submenu-row-name") {{ item.name }}
                  p(
                    class="submenu-row-attached"
                    :title="item.attachedButtons || 'No buttons'"
                  ) {{ formatAttachedButtons(item.attachedButtons) }}

                  button(v-if="editable" class="submenu-remove-btn" @click.stop="removeAction(item)")
                    MdiRemove(class="w-4.5 h-4.5")
</template>

<script setup lang="ts">
import MdiRemove from "~icons/mdi/remove?width=24px&height=24px";
import HugeiconsArrowDown01 from "~icons/hugeicons/arrow-down-01?width=24px&height=24px";

import { ref } from "vue";
const router = useRouter();
const { showContextMenu } = useGlobalContextMenu();

const props = defineProps({
  items: {
    type: Array,
    default: [],
  },
  actionType: {
    type: String,
    required: true,
  },
  routeTo: {
    type: String,
    default: "",
  },
  editable: {
    type: Boolean,
    default: true,
  },
});

const items2 = computed(() => {
  return props.items;
});

const isOpen = ref(false);
const submenuContent = ref<HTMLElement | null>(null);

const emit = defineEmits(["removeAction"]);

const removeAction = async (item: any) => {
  emit("removeAction", item, props.actionType);
};

const openItem = (item: any) => {
  if (!props.editable || !props.routeTo) return;
  router.push({
    path: props.routeTo,
    query: { actionId: item.id, integrationType: props.actionType },
  });
};

const getExecutionReferenceId = (item: any): string | number => {
  if (item.stable_id) {
    if (!item.stable_id) throw new Error("Saved integration action is missing stable_id");
    return item.stable_id;
  }
  return item.id;
};

const formatAttachedButtons = (attachedButtons: string | null | undefined) => {
  if (!attachedButtons || typeof attachedButtons !== "string") return "No buttons";
  const first = attachedButtons
    .split(",")
    .map((part) => part.trim())
    .filter(Boolean)[0];
  if (!first) return "No buttons";
  return first.length > 10 ? `${first.slice(0, 10)}...` : first;
};

const preflightLabel = (status: string) => ({
  ready: "Ready",
  disconnected: "Offline",
  missing_integration: "Missing",
  invalid_action: "Repair",
}[status] || "Check");

const preflightTitle = (preflight: any) =>
  preflight.issues?.map((issue: any) => issue.message).join("; ") || "Action is ready";

const handleContextMenu = (event: MouseEvent, item: any) => {
  event.preventDefault();
  showContextMenu(event, [
    {
      id: 'edit',
      label: 'Edit',
      action: () => router.push({path: props.routeTo, query: {actionId: item.id, integrationType: props.actionType}})
    },
    {
      id: 'copy',
      label: 'Copy/Duplicate',
      action: () => router.push({path: props.routeTo, query: {copyFrom: item.id, integrationType: props.actionType}})
    },
    {
      id: 'delete',
      label: 'Delete',
      action: () => removeAction(item)
    }
  ]);
};

function enter(el: HTMLElement, done: () => void) {
  // Get the actual height of the content
  el.style.opacity = "0";
  el.style.height = "auto";
  const height = el.offsetHeight + "px";
  el.style.height = "0";

  requestAnimationFrame(() => {
    el.style.transition = "all 0.2s ease-in-out";
    el.style.opacity = "1";
    el.style.height = height;

    el.addEventListener(
      "transitionend",
      () => {
        el.style.height = "auto"; // Set back to auto after animation
        done();
      },
      { once: true }
    );
  });
}

function leave(el: HTMLElement, done: () => void) {
  // Store the current height before transitioning
  const height = el.offsetHeight;
  el.style.height = height + "px";

  requestAnimationFrame(() => {
    el.style.transition = "all 0.2s ease-in-out";
    el.style.opacity = "0";
    el.style.height = "0";

    el.addEventListener(
      "transitionend",
      () => {
        el.style.display = "none"; // Hide element after animation
        done();
      },
      { once: true }
    );
  });
}
</script>

<style lang="scss" scoped>
.deck-submenu {
  --submenu-border: hsl(var(--border2) / 0.72);
  --submenu-border-2: hsl(var(--border2));
  --submenu-bg: hsl(var(--editor-surface-soft, 210 33% 98%));
  --submenu-bg-hover: hsl(var(--hover));
  --submenu-text: hsl(var(--text));
  --submenu-muted: hsl(var(--text2));

  .submenu-header {
    min-height: 3.1rem;
    border: 1px solid var(--submenu-border);
    background: var(--submenu-bg);
    transition: border-color 0.2s ease, background 0.2s ease;
    cursor: pointer;
  }

  .submenu-header:hover {
    border-color: hsl(var(--border2) / 0.8);
    background: var(--submenu-bg-hover);
  }

  .submenu-header :deep(p) {
    font-size: 1.02rem;
    font-weight: 700;
    color: var(--submenu-text);
    line-height: 1.15;
  }

  .submenu-header :deep(svg) {
    color: hsl(var(--text2));
  }

  .submenu-header :deep(.deck-mini-btn) {
    margin-left: auto;
  }

  .submenu-row {
    border: 1px solid var(--submenu-border-2);
    background: hsl(var(--background));
    border-radius: 0.62rem;
    min-height: 2.45rem;
    padding: 0.42rem 0.55rem;
    display: flex;
    align-items: center;
    gap: 0.45rem;
    transition: background 0.2s ease, border-color 0.2s ease, transform 0.2s ease;
    cursor: grab;
  }

  .submenu-row:hover {
    background: var(--submenu-bg-hover);
    border-color: hsl(var(--border2) / 0.85);
    transform: translateY(-1px);
  }
}

.submenu-content {
  border-left: none;
  margin-left: 0;
  padding-left: 0;
  margin-top: 0.2rem;
}

.submenu-list {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  padding-left: 0;
}

.submenu-row-name {
  flex: 1 1 auto;
  min-width: 0;
  font-size: 0.83rem;
  color: hsl(var(--text));
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.submenu-row-attached {
  font-size: 0.67rem;
  font-weight: 700;
  color: hsl(var(--text3));
  max-width: 4.5rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  flex-shrink: 0;
}

.submenu-health {
  border-radius: 999px;
  padding: 0.13rem 0.38rem;
  font-size: 0.61rem;
  font-weight: 800;
  color: hsl(var(--text2));
  background: hsl(var(--background3));
  flex-shrink: 0;
}

.submenu-health--ready {
  color: hsl(var(--success));
  background: hsl(var(--success) / 0.12);
}

.submenu-health--disconnected {
  color: hsl(var(--warning));
  background: hsl(var(--warning) / 0.12);
}

.submenu-health--missing_integration,
.submenu-health--invalid_action {
  color: hsl(var(--error));
  background: hsl(var(--error) / 0.12);
}

.submenu-remove-btn {
  width: 1.85rem;
  height: 1.85rem;
  border-radius: 0.45rem;
  border: none;
  background: transparent;
  color: hsl(var(--text3));
  visibility: hidden;
  opacity: 0;
  transition: background 0.2s ease, color 0.2s ease, opacity 0.2s ease;
}

.submenu-row:hover .submenu-remove-btn {
  visibility: visible;
  opacity: 1;
}

.submenu-remove-btn:hover {
  background: hsl(var(--error) / 0.1);
  color: hsl(var(--error));
}

.submenu-empty {
  font-size: 0.76rem;
  color: hsl(var(--text3));
  padding: 0.45rem 0.55rem;
}

.hide-left {
  animation: hide-left 0.5s ease-in-out forwards;

  @media (min-width: 1024px) {
    animation: none;
  }
}
@keyframes hide-left {
  from {
    transform: translateX(0rem);
  }

  to {
    transform: translateX(-100%);
  }
}
.show-right {
  animation: show-right 0.5s ease-in-out forwards;

  @media (min-width: 1024px) {
    animation: none;
  }
}

@keyframes show-right {
  from {
    transform: translateX(-25rem);
  }
  to {
    transform: translateX(0rem);
  }
}
.submenu {
  will-change: opacity, height;
  overflow: hidden; /* Ensure content outside the height is hidden */
  height: 0; /* Start with 0 height to hide the submenu initially */
  opacity: 0; /* Start with 0 opacity to make it invisible */
}

.submenu-enter-active {
  transition: all 0.2s ease-in-out;
}

.submenu-leave-active {
  transition: all 0.2s ease-in-out;
}

::-webkit-scrollbar {
  display: block;
}
</style>
