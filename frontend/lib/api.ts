import { z } from "zod";

import type { ApiError, Conversation, Message, StreamEvent } from "@/lib/types";

const API_BASE_URL = process.env.NEXT_PUBLIC_API_BASE_URL ?? "http://127.0.0.1:8080";

const conversationSchema = z.object({
  id: z.string(),
  title: z.string(),
  created_at: z.string(),
  updated_at: z.string(),
});

const messageSchema = z.object({
  id: z.string(),
  conversation_id: z.string(),
  role: z.enum(["user", "assistant", "system"]),
  content: z.string(),
  created_at: z.string(),
});

const apiErrorSchema = z.object({
  error: z.object({
    code: z.string(),
    message: z.string(),
    details: z.unknown().optional(),
  }),
});

const conversationListSchema = z.object({
  conversations: z.array(conversationSchema),
});

const createConversationSchema = z.object({
  conversation: conversationSchema,
});

const messageListSchema = z.object({
  messages: z.array(messageSchema),
});

const streamEventSchema: z.ZodType<StreamEvent> = z.discriminatedUnion("type", [
  z.object({ type: z.literal("user_message"), message: messageSchema }),
  z.object({ type: z.literal("chunk"), content: z.string() }),
  z.object({ type: z.literal("assistant_message"), message: messageSchema }),
  z.object({ type: z.literal("done") }),
  z.object({ type: z.literal("error"), code: z.string(), message: z.string() }),
]);

export async function listConversations(): Promise<Conversation[]> {
  const json = await requestJson(`${API_BASE_URL}/api/conversations`);
  return conversationListSchema.parse(json).conversations;
}

export async function createConversation(title?: string): Promise<Conversation> {
  const json = await requestJson(`${API_BASE_URL}/api/conversations`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(title === undefined ? {} : { title }),
  });
  return createConversationSchema.parse(json).conversation;
}

export async function listMessages(conversationId: string): Promise<Message[]> {
  const json = await requestJson(
    `${API_BASE_URL}/api/conversations/${conversationId}/messages`,
  );
  return messageListSchema.parse(json).messages;
}

export type StreamHandlers = {
  onUserMessage?: (message: Message) => void;
  onChunk?: (content: string) => void;
  onAssistantMessage?: (message: Message) => void;
  onDone?: () => void;
};

export async function sendMessageStream(
  conversationId: string,
  content: string,
  handlers: StreamHandlers,
): Promise<void> {
  const response = await fetch(
    `${API_BASE_URL}/api/conversations/${conversationId}/messages/stream`,
    {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ content }),
    },
  );

  if (!response.ok) {
    throw new Error(await readApiError(response));
  }

  if (!response.body) {
    throw new Error("Streaming response was empty.");
  }

  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  let buffer = "";

  for (;;) {
    const { value, done } = await reader.read();
    if (done) {
      break;
    }
    buffer += decoder.decode(value, { stream: true });
    buffer = processLines(buffer, handlers);
  }

  buffer += decoder.decode();
  processLines(`${buffer}\n`, handlers);
}

async function requestJson(url: string, init?: RequestInit): Promise<unknown> {
  const response = await fetch(url, init);
  if (!response.ok) {
    throw new Error(await readApiError(response));
  }
  return response.json();
}

async function readApiError(response: Response): Promise<string> {
  try {
    const json = await response.json();
    const parsed = apiErrorSchema.safeParse(json);
    if (parsed.success) {
      return parsed.data.error.message;
    }
  } catch {
    // Fall back to a status message below.
  }
  return `Request failed with ${response.status}`;
}

function processLines(buffer: string, handlers: StreamHandlers): string {
  const lines = buffer.split("\n");
  const rest = lines.pop() ?? "";

  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed) {
      continue;
    }
    const event = streamEventSchema.parse(JSON.parse(trimmed));
    handleStreamEvent(event, handlers);
  }

  return rest;
}

function handleStreamEvent(event: StreamEvent, handlers: StreamHandlers) {
  switch (event.type) {
    case "user_message":
      handlers.onUserMessage?.(event.message);
      break;
    case "chunk":
      handlers.onChunk?.(event.content);
      break;
    case "assistant_message":
      handlers.onAssistantMessage?.(event.message);
      break;
    case "done":
      handlers.onDone?.();
      break;
    case "error":
      throw new Error(event.message);
  }
}

export function isApiError(value: unknown): value is ApiError {
  return apiErrorSchema.safeParse(value).success;
}
