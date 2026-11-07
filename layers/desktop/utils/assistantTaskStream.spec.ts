import { describe, expect, it } from 'vitest';
import { isCurrentAssistantTaskChunk, type AssistantTaskChunk } from './assistantTaskStream';

const chunk = (overrides: Partial<AssistantTaskChunk> = {}): AssistantTaskChunk => ({
  taskId: 'task-a',
  conversationId: 'conversation-a',
  sequence: 1,
  content: 'hello',
  done: false,
  cancelled: false,
  ...overrides,
});

describe('assistant task stream routing', () => {
  it('accepts only the current task in the current conversation', () => {
    expect(isCurrentAssistantTaskChunk(chunk(), 'task-a', 'conversation-a', 0)).toBe(true);
    expect(isCurrentAssistantTaskChunk(chunk({ taskId: 'task-b' }), 'task-a', 'conversation-a', 0)).toBe(false);
    expect(isCurrentAssistantTaskChunk(chunk({ conversationId: 'conversation-b' }), 'task-a', 'conversation-a', 0)).toBe(false);
  });

  it('rejects replayed and out-of-order progress', () => {
    expect(isCurrentAssistantTaskChunk(chunk({ sequence: 4 }), 'task-a', 'conversation-a', 4)).toBe(false);
    expect(isCurrentAssistantTaskChunk(chunk({ sequence: 3 }), 'task-a', 'conversation-a', 4)).toBe(false);
    expect(isCurrentAssistantTaskChunk(chunk({ sequence: Number.NaN }), 'task-a', 'conversation-a', 0)).toBe(false);
  });
});
