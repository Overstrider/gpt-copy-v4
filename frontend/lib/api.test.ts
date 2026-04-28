import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { createConversation, sendMessageStream } from "@/lib/api";

const conversation = {
  id: "c1",
  title: "New chat",
  created_at: "2026-04-27T00:00:00.000Z",
  updated_at: "2026-04-27T00:00:00.000Z",
};

describe("api client", () => {
  beforeEach(() => {
    vi.stubGlobal("fetch", vi.fn());
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("preserves blank conversation titles for backend validation", async () => {
    const fetchMock = vi.mocked(fetch);
    fetchMock.mockResolvedValueOnce(
      new Response(JSON.stringify({ conversation }), {
        status: 201,
        headers: { "content-type": "application/json" },
      }),
    );

    await createConversation("");

    expect(fetchMock).toHaveBeenCalledWith(
      "http://127.0.0.1:8080/api/conversations",
      expect.objectContaining({
        body: JSON.stringify({ title: "" }),
      }),
    );
  });

  it("rejects a truncated stream without a done event", async () => {
    const fetchMock = vi.mocked(fetch);
    const onChunk = vi.fn();
    fetchMock.mockResolvedValueOnce(streamResponse({ type: "chunk", content: "partial" }));

    await expect(
      sendMessageStream("c1", "hello", {
        onChunk,
      }),
    ).rejects.toThrow("Streaming response ended before completion.");

    expect(onChunk).toHaveBeenCalledWith("partial");
  });

  it("resolves when the stream includes the done event", async () => {
    const fetchMock = vi.mocked(fetch);
    const onDone = vi.fn();
    fetchMock.mockResolvedValueOnce(
      streamResponse({ type: "chunk", content: "complete" }, { type: "done" }),
    );

    await sendMessageStream("c1", "hello", {
      onDone,
    });

    expect(onDone).toHaveBeenCalledOnce();
  });
});

function streamResponse(...events: unknown[]): Response {
  const body = events.map((event) => JSON.stringify(event)).join("\n");
  const stream = new ReadableStream({
    start(controller) {
      controller.enqueue(new TextEncoder().encode(body));
      controller.close();
    },
  });

  return new Response(stream, {
    status: 200,
    headers: { "content-type": "application/x-ndjson" },
  });
}
