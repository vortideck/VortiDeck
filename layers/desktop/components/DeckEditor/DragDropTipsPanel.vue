<template lang="pug">
div(class="drag-tips-panel")
  div(class="drag-tips-hero")
    p(class="drag-tips-kicker") Mini tutorial
    h3(class="drag-tips-title") Drag assignments anywhere in your workflow
    p(class="drag-tips-subtitle")
      | Drag from the left Assignments list and drop into the destination you want to configure.

  div(class="drag-walkthrough-shell")
    div(class="drag-walkthrough-header")
      div
        p(class="drag-tips-steps-label") Guided mode
        p(class="drag-walkthrough-hint") Follow step-by-step with focused previews
      button(
        type="button"
        class="drag-walkthrough-toggle"
        @click="toggleWalkthrough"
      ) {{ showWalkthrough ? 'Hide Walkthrough' : 'Start Walkthrough' }}

    Transition(name="walkthrough-expand")
      div(v-if="showWalkthrough" class="drag-walkthrough-body")
        div(class="drag-step-chips")
          button(
            v-for="(step, index) in walkthroughSteps"
            :key="step.id"
            type="button"
            class="drag-step-chip"
            :class="activeStep === index ? 'drag-step-chip--active' : ''"
            @click="activeStep = index"
          ) {{ index + 1 }}. {{ step.label }}

        div(class="drag-wizard-card")
          video(
            class="drag-wizard-video"
            :key="currentStep.id"
            :src="currentStep.video"
            :style="{ objectPosition: currentStep.position }"
            autoplay
            loop
            muted
            playsinline
            preload="metadata"
          )
          div(class="drag-wizard-copy")
            p(class="drag-wizard-kicker") Step {{ activeStep + 1 }} / {{ walkthroughSteps.length }}
            p(class="drag-wizard-title") {{ currentStep.title }}
            p(class="drag-wizard-description") {{ currentStep.description }}

          div(class="drag-wizard-actions")
            button(type="button" class="drag-wizard-btn drag-wizard-btn--ghost" :disabled="activeStep === 0" @click="prevStep") Previous
            button(type="button" class="drag-wizard-btn drag-wizard-btn--primary" :disabled="activeStep === walkthroughSteps.length - 1" @click="nextStep") Next

  div(class="drag-tips-targets")
    div(class="drag-target-card")
      div(class="drag-target-head")
        span(class="drag-target-dot")
        p(class="drag-target-title") Deck Buttons
      p(class="drag-target-text") Drop on a button in Deck Simulator to bind the action instantly.

    div(class="drag-target-card")
      div(class="drag-target-head")
        span(class="drag-target-dot")
        p(class="drag-target-title") Context Menu
      p(class="drag-target-text") Drop in Context Menu panel to create a Windows right-click entry.

    div(class="drag-target-card")
      div(class="drag-target-head")
        span(class="drag-target-dot")
        p(class="drag-target-title") Streaming Assignments
      p(class="drag-target-text") Drop into streaming assignment slots for chat-triggered controls.

    div(class="drag-target-card")
      div(class="drag-target-head")
        span(class="drag-target-dot")
        p(class="drag-target-title") Flow Builder
      p(class="drag-target-text") Drop into Flow Builder to create connected automation nodes.

  div(class="drag-tips-steps")
    p(class="drag-tips-steps-label") Quick flow
    ol(class="drag-steps-list")
      li(class="drag-step-item")
        span(class="drag-step-index") 1
        span Pick any assignment (Macro, Program, Flow, Action, Utility).
      li(class="drag-step-item")
        span(class="drag-step-index") 2
        span Drag until the destination highlights.
      li(class="drag-step-item")
        span(class="drag-step-index") 3
        span Drop to assign, then test from that destination.
</template>

<script setup lang="ts">
const showWalkthrough = ref(false);
const activeStep = ref(0);

const walkthroughSteps = [
  {
    id: "create-assignment",
    label: "Create Assignment",
    title: "Create or prepare the assignment",
    description: "Set up the program/action first so it can be dragged into the destination panel.",
    video: "/create_program.webm",
    position: "50% 50%",
  },
  {
    id: "drag-assignment",
    label: "Drag",
    title: "Drag assignment to the target panel",
    description: "Drag from Assignments and keep moving until the drop state appears.",
    video: "/dreag.webm",
    position: "50% 50%",
  },
  {
    id: "drop-and-create",
    label: "Drop and Create",
    title: "Drop and create the context item",
    description: "Drop into Context Menu panel, review the prefilled form, then save.",
    video: "context.webm",
    position: "50% 50%",
  },
  {
    id: "test-it",
    label: "Test it",
    title: "Test the created item",
    description: "Open Explorer right-click menu and run the new VortiDeck action.",
    video: "/use_context.webm",
    position: "50% 50%",
  },
];

const currentStep = computed(() => walkthroughSteps[activeStep.value] || walkthroughSteps[0]);

const toggleWalkthrough = () => {
  showWalkthrough.value = !showWalkthrough.value;
  if (showWalkthrough.value) activeStep.value = 0;
};

const prevStep = () => {
  if (activeStep.value > 0) activeStep.value -= 1;
};

const nextStep = () => {
  if (activeStep.value < walkthroughSteps.length - 1) activeStep.value += 1;
};
</script>

<style scoped lang="scss">
.drag-tips-panel {
  display: flex;
  flex-direction: column;
  gap: 0.9rem;
}

.drag-tips-hero {
  border: 1px solid hsl(var(--border2) / 0.7);
  border-radius: 0.85rem;
  background: hsl(var(--background2) / 0.65);
  padding: 0.95rem 1rem;
}

.drag-tips-kicker {
  font-size: 0.62rem;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: hsl(var(--text2) / 0.78);
  font-weight: 700;
}

.drag-tips-title {
  margin-top: 0.25rem;
  font-size: 1rem;
  font-weight: 800;
  color: hsl(var(--text));
}

.drag-tips-subtitle {
  margin-top: 0.35rem;
  font-size: 0.78rem;
  line-height: 1.35;
  color: hsl(var(--text2) / 0.9);
}

.drag-tips-targets {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.65rem;
}

.drag-target-card {
  border: 1px solid hsl(var(--border2) / 0.65);
  border-radius: 0.75rem;
  background: hsl(var(--background2) / 0.5);
  padding: 0.7rem 0.75rem;
}

.drag-target-head {
  display: flex;
  align-items: center;
  gap: 0.45rem;
}

.drag-target-dot {
  width: 0.44rem;
  height: 0.44rem;
  border-radius: 999px;
  background: hsl(var(--editor-accent));
  box-shadow: 0 0 0 4px hsl(var(--editor-accent) / 0.14);
}

.drag-target-title {
  font-size: 0.76rem;
  font-weight: 700;
  color: hsl(var(--text));
}

.drag-target-text {
  margin-top: 0.36rem;
  font-size: 0.7rem;
  line-height: 1.35;
  color: hsl(var(--text2) / 0.86);
}

.drag-tips-steps {
  border: 1px solid hsl(var(--border2) / 0.65);
  border-radius: 0.75rem;
  background: hsl(var(--background2) / 0.44);
  padding: 0.75rem 0.8rem;
}

.drag-tips-steps-label {
  font-size: 0.64rem;
  letter-spacing: 0.11em;
  text-transform: uppercase;
  color: hsl(var(--text2) / 0.7);
  font-weight: 700;
  margin-bottom: 0.45rem;
}

.drag-steps-list {
  display: flex;
  flex-direction: column;
  gap: 0.42rem;
}

.drag-step-item {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  font-size: 0.72rem;
  color: hsl(var(--text2) / 0.9);
}

.drag-step-index {
  width: 1rem;
  height: 1rem;
  border-radius: 999px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-size: 0.6rem;
  font-weight: 700;
  color: hsl(var(--editor-accent));
  border: 1px solid hsl(var(--editor-accent) / 0.35);
  background: hsl(var(--editor-accent) / 0.08);
  flex-shrink: 0;
}

.drag-walkthrough-shell {
  border: 1px solid hsl(var(--border2) / 0.65);
  border-radius: 0.78rem;
  background: hsl(var(--background2) / 0.54);
  padding: 0.7rem;
}

.drag-walkthrough-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.8rem;
}

.drag-walkthrough-hint {
  font-size: 0.72rem;
  color: hsl(var(--text2) / 0.8);
  margin-top: 0.12rem;
}

.drag-walkthrough-toggle {
  border: 1px solid hsl(var(--editor-accent) / 0.42);
  color: hsl(var(--editor-accent));
  background: hsl(var(--editor-accent) / 0.08);
  border-radius: 0.55rem;
  padding: 0.38rem 0.62rem;
  font-size: 0.72rem;
  font-weight: 700;
  transition: background 0.18s ease, border-color 0.18s ease;
}

.drag-walkthrough-toggle:hover {
  background: hsl(var(--editor-accent) / 0.14);
  border-color: hsl(var(--editor-accent) / 0.58);
}

.drag-walkthrough-body {
  margin-top: 0.7rem;
  display: flex;
  flex-direction: column;
  gap: 0.64rem;
}

.drag-step-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 0.42rem;
}

.drag-step-chip {
  border: 1px solid hsl(var(--border2) / 0.68);
  color: hsl(var(--text2) / 0.88);
  background: hsl(var(--background2) / 0.45);
  border-radius: 999px;
  padding: 0.25rem 0.55rem;
  font-size: 0.68rem;
  font-weight: 600;
  transition: color 0.18s ease, border-color 0.18s ease, background 0.18s ease;
}

.drag-step-chip--active {
  border-color: hsl(var(--editor-accent) / 0.52);
  background: hsl(var(--editor-accent) / 0.14);
  color: hsl(var(--editor-accent));
}

.drag-wizard-card {
  border: 1px solid hsl(var(--border2) / 0.68);
  border-radius: 0.65rem;
  background: hsl(var(--background2) / 0.46);
  padding: 0.6rem;
}

.drag-wizard-video {
  width: 100%;
  aspect-ratio: 16 / 9;
  border-radius: 0.48rem;
  border: 1px solid hsl(var(--border2) / 0.72);
  background: hsl(var(--background) / 0.8);
  object-fit: cover;
}

.drag-wizard-copy {
  margin-top: 0.55rem;
}

.drag-wizard-kicker {
  font-size: 0.62rem;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: hsl(var(--text2) / 0.76);
  font-weight: 700;
}

.drag-wizard-title {
  margin-top: 0.18rem;
  font-size: 0.84rem;
  font-weight: 700;
  color: hsl(var(--text));
}

.drag-wizard-description {
  margin-top: 0.22rem;
  font-size: 0.73rem;
  line-height: 1.35;
  color: hsl(var(--text2) / 0.88);
}

.drag-wizard-actions {
  margin-top: 0.55rem;
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 0.45rem;
}

.drag-wizard-btn {
  border-radius: 0.5rem;
  font-size: 0.7rem;
  font-weight: 700;
  padding: 0.33rem 0.62rem;
  transition: filter 0.18s ease, opacity 0.18s ease;
}

.drag-wizard-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.drag-wizard-btn--ghost {
  border: 1px solid hsl(var(--border2) / 0.75);
  color: hsl(var(--text2) / 0.95);
  background: hsl(var(--background2) / 0.52);
}

.drag-wizard-btn--primary {
  border: 1px solid hsl(var(--editor-accent) / 0.58);
  color: hsl(var(--editor-accent));
  background: hsl(var(--editor-accent) / 0.12);
}

.walkthrough-expand-enter-active,
.walkthrough-expand-leave-active {
  transition: all 0.2s ease;
}

.walkthrough-expand-enter-from,
.walkthrough-expand-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}

@media (max-width: 1100px) {
  .drag-tips-targets {
    grid-template-columns: 1fr;
  }
}
</style>
