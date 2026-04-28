export type Conversation = {
  id: string;
  title: string;
  created_at: string;
  updated_at: string;
};

export type Message = {
  id: string;
  conversation_id: string;
  role: "user" | "assistant" | "system";
  content: string;
  created_at: string;
};

export type ApiError = {
  error: {
    code: string;
    message: string;
    details?: unknown;
  };
};

export type StreamEvent =
  | { type: "user_message"; message: Message }
  | { type: "chunk"; content: string }
  | { type: "assistant_message"; message: Message }
  | { type: "done" }
  | { type: "error"; code: string; message: string };
