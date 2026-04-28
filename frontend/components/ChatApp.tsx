"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  AlertCircle,
  LoaderCircle,
  Menu,
  MessageSquare,
  PanelLeftClose,
  Plus,
  Send,
} from "lucide-react";
import React, { FormEvent, useEffect, useMemo, useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

import {
  createConversation,
  listConversations,
  listMessages,
  sendMessageStream,
} from "@/lib/api";
import type { Conversation, Message } from "@/lib/types";

const emptyConversations: Conversation[] = [];
const emptyMessages: Message[] = [];

export function ChatApp() {
  const queryClient = useQueryClient();
  const [selectedConversationId, setSelectedConversationId] = useState<string | null>(null);
  const [draft, setDraft] = useState("");
  const [isSidebarOpen, setIsSidebarOpen] = useState(false);
  const [isSending, setIsSending] = useState(false);
  const [pendingTurn, setPendingTurn] = useState<{
    conversationId: string;
    optimisticMessages: Message[];
    streamingMessage: Message | null;
  } | null>(null);
  const [error, setError] = useState<string | null>(null);

  const conversationsQuery = useQuery({
    queryKey: ["conversations"],
    queryFn: listConversations,
  });

  const conversations = useMemo(
    () => conversationsQuery.data ?? emptyConversations,
    [conversationsQuery.data],
  );

  useEffect(() => {
    if (!selectedConversationId && conversations.length > 0) {
      setSelectedConversationId(conversations[0].id);
    }
  }, [conversations, selectedConversationId]);

  useEffect(() => {
    setError(null);
  }, [selectedConversationId]);

  const messagesQuery = useQuery({
    queryKey: ["messages", selectedConversationId],
    queryFn: () => listMessages(selectedConversationId ?? ""),
    enabled: Boolean(selectedConversationId),
  });

  const createConversationMutation = useMutation({
    mutationFn: createConversation,
    onSuccess: async (conversation) => {
      setSelectedConversationId(conversation.id);
      await queryClient.invalidateQueries({ queryKey: ["conversations"] });
    },
  });

  const selectedConversation = useMemo(
    () => conversations.find((conversation) => conversation.id === selectedConversationId),
    [conversations, selectedConversationId],
  );

  const baseMessages = messagesQuery.data ?? emptyMessages;
  const activePendingTurn =
    pendingTurn?.conversationId === selectedConversationId ? pendingTurn : null;
  const displayedMessages = activePendingTurn?.streamingMessage
    ? [
        ...baseMessages,
        ...activePendingTurn.optimisticMessages,
        activePendingTurn.streamingMessage,
      ]
    : [...baseMessages, ...(activePendingTurn?.optimisticMessages ?? [])];

  async function handleNewConversation() {
    setError(null);
    const conversation = await createConversationMutation.mutateAsync(undefined);
    setSelectedConversationId(conversation.id);
    setIsSidebarOpen(false);
  }

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const content = draft.trim();
    if (!content || isSending) {
      return;
    }

    setDraft("");
    setError(null);
    setIsSending(true);
    setPendingTurn(null);

    let activeConversationId = selectedConversationId;

    try {
      if (!activeConversationId) {
        const conversation = await createConversation(undefined);
        activeConversationId = conversation.id;
        setSelectedConversationId(conversation.id);
        await queryClient.invalidateQueries({ queryKey: ["conversations"] });
      }

      const conversationId = activeConversationId;
      setPendingTurn({
        conversationId,
        optimisticMessages: [],
        streamingMessage: null,
      });
      await sendMessageStream(conversationId, content, {
        onUserMessage: (message) => {
          setPendingTurn((current) => ({
            conversationId,
            optimisticMessages: [message],
            streamingMessage:
              current?.conversationId === conversationId ? current.streamingMessage : null,
          }));
        },
        onChunk: (chunk) => {
          setPendingTurn((current) => {
            const streamingMessage =
              current?.conversationId === conversationId ? current.streamingMessage : null;

            return {
              conversationId,
              optimisticMessages:
                current?.conversationId === conversationId ? current.optimisticMessages : [],
              streamingMessage: {
                id: streamingMessage?.id ?? "streaming-assistant",
                conversation_id: conversationId,
                role: "assistant",
                content: `${streamingMessage?.content ?? ""}${chunk}`,
                created_at: streamingMessage?.created_at ?? new Date().toISOString(),
              },
            };
          });
        },
        onAssistantMessage: (message) => {
          setPendingTurn((current) => ({
            conversationId,
            optimisticMessages: [
              ...(current?.conversationId === conversationId
                ? current.optimisticMessages.filter((item) => item.role === "user")
                : []),
            message,
            ],
            streamingMessage: null,
          }));
        },
      });

      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["conversations"] }),
        queryClient.invalidateQueries({ queryKey: ["messages", conversationId] }),
      ]);
      setPendingTurn(null);
    } catch (caught) {
      if (activeConversationId) {
        await Promise.all([
          queryClient.invalidateQueries({ queryKey: ["conversations"] }),
          queryClient.invalidateQueries({ queryKey: ["messages", activeConversationId] }),
        ]);
      }
      setPendingTurn(null);
      setError(caught instanceof Error ? caught.message : "Message failed to send.");
    } finally {
      setIsSending(false);
    }
  }

  return (
    <main className="flex h-dvh bg-[#f6f5f2] text-[#151514]">
      <ConversationSidebar
        conversations={conversations}
        selectedConversationId={selectedConversationId}
        isOpen={isSidebarOpen}
        isLoading={conversationsQuery.isLoading}
        isBusy={isSending || createConversationMutation.isPending}
        onSelect={(conversationId) => {
          setSelectedConversationId(conversationId);
          setIsSidebarOpen(false);
        }}
        onNewConversation={handleNewConversation}
        onClose={() => setIsSidebarOpen(false)}
      />

      <section className="flex min-w-0 flex-1 flex-col">
        <header className="flex h-14 shrink-0 items-center gap-3 border-b border-[#ded9cf] bg-[#fbfaf8]/90 px-3 backdrop-blur sm:px-5">
          <button
            type="button"
            className="inline-flex h-10 w-10 items-center justify-center rounded-md border border-[#d4cec1] bg-white text-[#4b4741] shadow-sm md:hidden"
            onClick={() => setIsSidebarOpen(true)}
            aria-label="Open conversations"
            title="Open conversations"
          >
            <Menu className="h-5 w-5" aria-hidden="true" />
          </button>
          <div className="min-w-0">
            <h1 className="truncate text-base font-semibold">
              {selectedConversation?.title ?? "gpt-copy-v4"}
            </h1>
          </div>
        </header>

        <div className="flex min-h-0 flex-1 flex-col">
          <Transcript
            messages={displayedMessages}
            isLoading={messagesQuery.isLoading}
            isSending={isSending}
            error={error}
          />
          <Composer
            draft={draft}
            isSending={isSending}
            onDraftChange={setDraft}
            onSubmit={handleSubmit}
          />
        </div>
      </section>
    </main>
  );
}

function ConversationSidebar({
  conversations,
  selectedConversationId,
  isOpen,
  isLoading,
  isBusy,
  onSelect,
  onNewConversation,
  onClose,
}: {
  conversations: Conversation[];
  selectedConversationId: string | null;
  isOpen: boolean;
  isLoading: boolean;
  isBusy: boolean;
  onSelect: (conversationId: string) => void;
  onNewConversation: () => void;
  onClose: () => void;
}) {
  return (
    <>
      <div
        className={`fixed inset-0 z-20 bg-black/30 transition-opacity md:hidden ${
          isOpen ? "opacity-100" : "pointer-events-none opacity-0"
        }`}
        onClick={onClose}
      />
      <aside
        className={`fixed inset-y-0 left-0 z-30 flex w-[18rem] max-w-[86vw] flex-col border-r border-[#d8d1c4] bg-[#25231f] text-[#f7f3eb] transition-transform md:static md:z-auto md:translate-x-0 ${
          isOpen ? "translate-x-0" : "-translate-x-full"
        }`}
      >
        <div className="flex h-14 shrink-0 items-center justify-between border-b border-white/10 px-3">
          <div className="flex min-w-0 items-center gap-2">
            <div className="flex h-8 w-8 items-center justify-center rounded-md bg-[#2f8f6b]">
              <MessageSquare className="h-4 w-4" aria-hidden="true" />
            </div>
            <span className="truncate text-sm font-semibold">gpt-copy-v4</span>
          </div>
          <button
            type="button"
            className="inline-flex h-9 w-9 items-center justify-center rounded-md text-[#e8e2d8] hover:bg-white/10 md:hidden"
            onClick={onClose}
            aria-label="Close conversations"
            title="Close conversations"
          >
            <PanelLeftClose className="h-5 w-5" aria-hidden="true" />
          </button>
        </div>

        <div className="p-3">
          <button
            type="button"
            className="flex h-10 w-full items-center justify-center gap-2 rounded-md border border-white/15 bg-white/8 px-3 text-sm font-medium hover:bg-white/14 disabled:cursor-not-allowed disabled:opacity-50"
            onClick={onNewConversation}
            disabled={isBusy}
          >
            <Plus className="h-4 w-4" aria-hidden="true" />
            New chat
          </button>
        </div>

        <nav className="min-h-0 flex-1 space-y-1 overflow-y-auto px-2 pb-3">
          {isLoading ? (
            <div className="flex items-center gap-2 px-3 py-2 text-sm text-[#c7c0b4]">
              <LoaderCircle className="h-4 w-4 animate-spin" aria-hidden="true" />
              Loading
            </div>
          ) : null}
          {conversations.map((conversation) => (
            <button
              type="button"
              key={conversation.id}
              className={`flex h-10 w-full items-center gap-2 rounded-md px-3 text-left text-sm disabled:cursor-not-allowed disabled:opacity-50 ${
                conversation.id === selectedConversationId
                  ? "bg-[#f7f3eb] text-[#25231f]"
                  : "text-[#e8e2d8] hover:bg-white/10"
              }`}
              onClick={() => onSelect(conversation.id)}
              disabled={isBusy}
            >
              <MessageSquare className="h-4 w-4 shrink-0" aria-hidden="true" />
              <span className="truncate">{conversation.title}</span>
            </button>
          ))}
        </nav>
      </aside>
    </>
  );
}

function Transcript({
  messages,
  isLoading,
  isSending,
  error,
}: {
  messages: Message[];
  isLoading: boolean;
  isSending: boolean;
  error: string | null;
}) {
  return (
    <div className="min-h-0 flex-1 overflow-y-auto">
      <div className="mx-auto flex min-h-full w-full max-w-3xl flex-col px-4 py-6 sm:px-6">
        {messages.length === 0 && !isLoading ? (
          <div className="flex flex-1 items-center justify-center py-16 text-center">
            <div>
              <div className="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-md bg-[#2f8f6b] text-white">
                <MessageSquare className="h-6 w-6" aria-hidden="true" />
              </div>
              <h2 className="text-xl font-semibold">gpt-copy-v4</h2>
              <p className="mt-2 max-w-md text-sm leading-6 text-[#6c665d]">
                Start a conversation and the backend will proxy the model call.
              </p>
            </div>
          </div>
        ) : null}

        {isLoading ? (
          <div className="flex items-center gap-2 py-6 text-sm text-[#6c665d]">
            <LoaderCircle className="h-4 w-4 animate-spin" aria-hidden="true" />
            Loading messages
          </div>
        ) : null}

        <div className="space-y-5">
          {messages.map((message) => (
            <MessageBubble key={message.id} message={message} />
          ))}
          {isSending ? (
            <div className="flex items-center gap-2 text-sm text-[#6c665d]">
              <LoaderCircle className="h-4 w-4 animate-spin" aria-hidden="true" />
              Thinking
            </div>
          ) : null}
          {error ? (
            <div className="flex items-start gap-2 rounded-md border border-[#e3b7ad] bg-[#fff4f1] px-3 py-2 text-sm text-[#8d2e20]">
              <AlertCircle className="mt-0.5 h-4 w-4 shrink-0" aria-hidden="true" />
              <span>{error}</span>
            </div>
          ) : null}
        </div>
      </div>
    </div>
  );
}

function MessageBubble({ message }: { message: Message }) {
  const isUser = message.role === "user";
  return (
    <article className={`flex ${isUser ? "justify-end" : "justify-start"}`}>
      <div
        className={`max-w-[min(44rem,92%)] rounded-lg px-4 py-3 text-sm leading-6 shadow-sm ${
          isUser
            ? "bg-[#2f8f6b] text-white"
            : "border border-[#ded9cf] bg-[#fbfaf8] text-[#25231f]"
        }`}
      >
        {isUser ? (
          <p className="whitespace-pre-wrap">{message.content}</p>
        ) : (
          <div className="markdown">
            <ReactMarkdown remarkPlugins={[remarkGfm]}>{message.content}</ReactMarkdown>
          </div>
        )}
      </div>
    </article>
  );
}

function Composer({
  draft,
  isSending,
  onDraftChange,
  onSubmit,
}: {
  draft: string;
  isSending: boolean;
  onDraftChange: (value: string) => void;
  onSubmit: (event: FormEvent<HTMLFormElement>) => void;
}) {
  return (
    <div className="border-t border-[#ded9cf] bg-[#fbfaf8] px-3 py-3 sm:px-5">
      <form
        className="mx-auto flex w-full max-w-3xl items-end gap-2 rounded-lg border border-[#d4cec1] bg-white p-2 shadow-sm"
        onSubmit={onSubmit}
      >
        <textarea
          className="max-h-40 min-h-11 flex-1 resize-none bg-transparent px-2 py-2 text-sm leading-6 outline-none placeholder:text-[#8a8378]"
          placeholder="Message gpt-copy-v4"
          value={draft}
          onChange={(event) => onDraftChange(event.target.value)}
          rows={1}
          aria-label="Message"
        />
        <button
          type="submit"
          className="inline-flex h-10 w-10 shrink-0 items-center justify-center rounded-md bg-[#25231f] text-white disabled:cursor-not-allowed disabled:bg-[#b9b2a7]"
          disabled={!draft.trim() || isSending}
          aria-label="Send message"
          title="Send message"
        >
          {isSending ? (
            <LoaderCircle className="h-4 w-4 animate-spin" aria-hidden="true" />
          ) : (
            <Send className="h-4 w-4" aria-hidden="true" />
          )}
        </button>
      </form>
    </div>
  );
}
