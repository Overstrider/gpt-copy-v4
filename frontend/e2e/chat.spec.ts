import { expect, test } from "@playwright/test";

const corsHeaders = {
  "access-control-allow-origin": "http://127.0.0.1:3000",
  "access-control-allow-methods": "GET,POST,OPTIONS",
  "access-control-allow-headers": "content-type",
};

test("sends a message through the chat UI", async ({ page }) => {
  let hasConversation = false;
  const messages: unknown[] = [];

  await page.route("**/api/**", async (route) => {
    const request = route.request();
    const url = new URL(request.url());

    if (request.method() === "OPTIONS") {
      await route.fulfill({ status: 204, headers: corsHeaders });
      return;
    }

    if (url.pathname === "/api/conversations" && request.method() === "GET") {
      await route.fulfill({
        status: 200,
        headers: { ...corsHeaders, "content-type": "application/json" },
        body: JSON.stringify({
          conversations: hasConversation
            ? [
                {
                  id: "c1",
                  title: "Smoke test",
                  created_at: "2026-04-27T00:00:00.000Z",
                  updated_at: "2026-04-27T00:00:00.000Z",
                },
              ]
            : [],
        }),
      });
      return;
    }

    if (url.pathname === "/api/conversations" && request.method() === "POST") {
      hasConversation = true;
      await route.fulfill({
        status: 201,
        headers: { ...corsHeaders, "content-type": "application/json" },
        body: JSON.stringify({
          conversation: {
            id: "c1",
            title: "New chat",
            created_at: "2026-04-27T00:00:00.000Z",
            updated_at: "2026-04-27T00:00:00.000Z",
          },
        }),
      });
      return;
    }

    if (url.pathname === "/api/conversations/c1/messages" && request.method() === "GET") {
      await route.fulfill({
        status: 200,
        headers: { ...corsHeaders, "content-type": "application/json" },
        body: JSON.stringify({ messages }),
      });
      return;
    }

    if (
      url.pathname === "/api/conversations/c1/messages/stream" &&
      request.method() === "POST"
    ) {
      const userMessage = {
        id: "u1",
        conversation_id: "c1",
        role: "user",
        content: "Hello from Playwright",
        created_at: "2026-04-27T00:00:00.000Z",
      };
      const assistantMessage = {
        id: "a1",
        conversation_id: "c1",
        role: "assistant",
        content: "Smoke **passed**",
        created_at: "2026-04-27T00:00:00.000Z",
      };
      messages.splice(0, messages.length, userMessage, assistantMessage);

      const body = [
        {
          type: "user_message",
          message: userMessage,
        },
        { type: "chunk", content: "Smoke " },
        { type: "chunk", content: "passed" },
        {
          type: "assistant_message",
          message: assistantMessage,
        },
        { type: "done" },
      ]
        .map((event) => JSON.stringify(event))
        .join("\n");

      await route.fulfill({
        status: 200,
        headers: { ...corsHeaders, "content-type": "application/x-ndjson" },
        body: `${body}\n`,
      });
      return;
    }

    await route.fulfill({
      status: 404,
      headers: { ...corsHeaders, "content-type": "application/json" },
      body: JSON.stringify({ error: { code: "not_found", message: "not found" } }),
    });
  });

  await page.goto("/");
  await page.getByRole("textbox", { name: "Message" }).fill("Hello from Playwright");
  await page.getByRole("button", { name: "Send message" }).click();

  await expect(page.getByText("Hello from Playwright")).toBeVisible();
  await expect(page.getByText("passed")).toBeVisible();
});
