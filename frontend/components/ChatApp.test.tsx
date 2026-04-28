import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import React from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { ChatApp } from "@/components/ChatApp";
import * as api from "@/lib/api";
import type { Conversation, Message } from "@/lib/types";

vi.mock("@/lib/api");

const now = "2026-04-27T00:00:00.000Z";

const conversation: Conversation = {
  id: "c1",
  title: "Planning",
  created_at: now,
  updated_at: now,
};

const secondConversation: Conversation = {
  id: "c2",
  title: "Archive",
  created_at: now,
  updated_at: now,
};

const assistantMessage: Message = {
  id: "m1",
  conversation_id: "c1",
  role: "assistant",
  content: "**Hello** from markdown",
  created_at: now,
};

function renderChat() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  return render(
    <QueryClientProvider client={queryClient}>
      <ChatApp />
    </QueryClientProvider>,
  );
}

describe("ChatApp", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(api.listConversations).mockResolvedValue([conversation]);
    vi.mocked(api.listMessages).mockResolvedValue([assistantMessage]);
    vi.mocked(api.createConversation).mockResolvedValue(conversation);
    vi.mocked(api.sendMessageStream).mockImplementation(async (_conversationId, content, handlers) => {
      handlers.onUserMessage?.({
        id: "user-1",
        conversation_id: "c1",
        role: "user",
        content,
        created_at: now,
      });
      handlers.onChunk?.("Hi ");
      handlers.onChunk?.("there");
      handlers.onAssistantMessage?.({
        id: "assistant-2",
        conversation_id: "c1",
        role: "assistant",
        content: "Hi **there**",
        created_at: now,
      });
      handlers.onDone?.();
    });
  });

  it("renders conversations and assistant markdown", async () => {
    renderChat();

    expect(await screen.findByRole("button", { name: /planning/i })).toBeInTheDocument();
    expect(await screen.findByText("Hello")).toBeInTheDocument();
    expect(screen.getByText("from markdown")).toBeInTheDocument();
  });

  it("streams a sent message through the composer", async () => {
    const user = userEvent.setup();
    vi.mocked(api.listMessages)
      .mockResolvedValueOnce([assistantMessage])
      .mockResolvedValue([
        assistantMessage,
        {
          id: "user-1",
          conversation_id: "c1",
          role: "user",
          content: "Hello model",
          created_at: now,
        },
        {
          id: "assistant-2",
          conversation_id: "c1",
          role: "assistant",
          content: "Hi **there**",
          created_at: now,
        },
      ]);
    renderChat();

    await screen.findByRole("button", { name: /planning/i });
    await user.type(screen.getByRole("textbox", { name: /^message$/i }), "Hello model");
    await user.click(screen.getByRole("button", { name: /send message/i }));

    await waitFor(() =>
      expect(api.sendMessageStream).toHaveBeenCalledWith(
        "c1",
        "Hello model",
        expect.any(Object),
      ),
    );
    expect(await screen.findByText("Hello model")).toBeInTheDocument();
    expect(await screen.findByText("there")).toBeInTheDocument();
  });

  it("shows a user-visible error when sending fails", async () => {
    vi.mocked(api.sendMessageStream).mockRejectedValueOnce(new Error("Provider unavailable"));
    const user = userEvent.setup();
    renderChat();

    await screen.findByRole("button", { name: /planning/i });
    await user.type(screen.getByRole("textbox", { name: /^message$/i }), "Please fail");
    await user.click(screen.getByRole("button", { name: /send message/i }));

    expect(await screen.findByText("Provider unavailable")).toBeInTheDocument();
  });

  it("removes partial assistant output and reloads persisted messages after a mid-stream error", async () => {
    const user = userEvent.setup();
    vi.mocked(api.listMessages)
      .mockResolvedValueOnce([assistantMessage])
      .mockResolvedValue([
        assistantMessage,
        {
          id: "user-1",
          conversation_id: "c1",
          role: "user",
          content: "Please fail after chunk",
          created_at: now,
        },
      ]);
    vi.mocked(api.sendMessageStream).mockImplementationOnce(
      async (_conversationId, content, handlers) => {
        handlers.onUserMessage?.({
          id: "user-1",
          conversation_id: "c1",
          role: "user",
          content,
          created_at: now,
        });
        handlers.onChunk?.("partial answer");
        throw new Error("Provider failed mid-stream");
      },
    );

    renderChat();

    await screen.findByRole("button", { name: /planning/i });
    await user.type(screen.getByRole("textbox", { name: /^message$/i }), "Please fail after chunk");
    await user.click(screen.getByRole("button", { name: /send message/i }));

    expect(await screen.findByText("Provider failed mid-stream")).toBeInTheDocument();
    await waitFor(() => expect(api.listMessages).toHaveBeenCalledTimes(2));
    expect(screen.getByText("Please fail after chunk")).toBeInTheDocument();
    expect(screen.queryByText("partial answer")).not.toBeInTheDocument();
  });

  it("reloads messages when sending fails before stream events arrive", async () => {
    const user = userEvent.setup();
    vi.mocked(api.listMessages)
      .mockResolvedValueOnce([assistantMessage])
      .mockResolvedValue([
        assistantMessage,
        {
          id: "user-early-fail",
          conversation_id: "c1",
          role: "user",
          content: "Persisted before event",
          created_at: now,
        },
      ]);
    vi.mocked(api.sendMessageStream).mockRejectedValueOnce(
      new Error("OpenRouter request failed"),
    );

    renderChat();

    await screen.findByRole("button", { name: /planning/i });
    await user.type(screen.getByRole("textbox", { name: /^message$/i }), "Persisted before event");
    await user.click(screen.getByRole("button", { name: /send message/i }));

    expect(await screen.findByText("OpenRouter request failed")).toBeInTheDocument();
    await waitFor(() => expect(api.listMessages).toHaveBeenCalledTimes(2));
    expect(screen.getByText("Persisted before event")).toBeInTheDocument();
  });

  it("prevents switching conversations while a stream is in flight", async () => {
    const user = userEvent.setup();
    let resolveStream: (() => void) | undefined;
    const streamFinished = new Promise<void>((resolve) => {
      resolveStream = resolve;
    });

    vi.mocked(api.listConversations).mockResolvedValue([conversation, secondConversation]);
    vi.mocked(api.sendMessageStream).mockImplementationOnce(
      async (_conversationId, content, handlers) => {
        handlers.onUserMessage?.({
          id: "user-inflight",
          conversation_id: "c1",
          role: "user",
          content,
          created_at: now,
        });
        handlers.onChunk?.("still streaming");
        await streamFinished;
        handlers.onAssistantMessage?.({
          id: "assistant-inflight",
          conversation_id: "c1",
          role: "assistant",
          content: "done",
          created_at: now,
        });
      },
    );

    renderChat();

    await screen.findByRole("button", { name: /planning/i });
    await user.type(screen.getByRole("textbox", { name: /^message$/i }), "Keep this scoped");
    await user.click(screen.getByRole("button", { name: /send message/i }));

    await waitFor(() => expect(api.sendMessageStream).toHaveBeenCalled());
    const archiveButton = screen.getByRole("button", { name: /archive/i });
    expect(archiveButton).toBeDisabled();

    await user.click(archiveButton);

    expect(screen.getByRole("heading", { name: /planning/i })).toBeInTheDocument();
    expect(screen.getByText("Keep this scoped")).toBeInTheDocument();
    expect(screen.getByText("still streaming")).toBeInTheDocument();

    resolveStream?.();
    await waitFor(() => expect(screen.queryByText("Thinking")).not.toBeInTheDocument());
  });
});
