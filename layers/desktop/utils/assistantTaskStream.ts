export interface AssistantTaskChunk {
  taskId: string;
  conversationId: string;
  sequence: number;
  content: string;
  done: boolean;
  cancelled: boolean;
}

export function isCurrentAssistantTaskChunk(
  chunk: AssistantTaskChunk,
  taskId: string | null,
  conversationId: string,
  lastSequence: number,
): boolean {
  return chunk.taskId === taskId
    && chunk.conversationId === conversationId
    && Number.isSafeInteger(chunk.sequence)
    && chunk.sequence > lastSequence;
}
