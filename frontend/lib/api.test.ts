import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { createConversation } from "@/lib/api";

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
});
