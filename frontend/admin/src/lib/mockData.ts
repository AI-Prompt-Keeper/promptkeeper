import type { FunctionRow, PromptVersion, ModelOption } from "@/types";

export const mockFunctions: FunctionRow[] = [
  { id: "fn-1", name: "customer_support_reply", activeVersion: "v12", avgLatencyMs: 342, costLast24h: 2.84 },
  { id: "fn-2", name: "content_summarizer", activeVersion: "v8", avgLatencyMs: 521, costLast24h: 1.22 },
  { id: "fn-3", name: "code_review_agent", activeVersion: "v5", avgLatencyMs: 890, costLast24h: 5.10 },
  { id: "fn-4", name: "ticket_classifier", activeVersion: "v3", avgLatencyMs: 128, costLast24h: 0.45 },
];

export const mockVersions: PromptVersion[] = [
  {
    id: "v1",
    functionId: "fn-1",
    createdAt: "2026-02-05T14:32:00Z",
    label: "v12",
    systemPrompt: "You are a helpful customer support agent. Be concise and empathetic.",
    userPrompt: "Customer message:\n\n{{user_input}}\n\nContext: {{context}}",
    modelConfig: { temperature: 0.7, max_tokens: 1024 },
    isProduction: true,
  },
  {
    id: "v2",
    functionId: "fn-1",
    createdAt: "2026-02-05T12:10:00Z",
    label: "v11",
    systemPrompt: "You are a support agent. Focus on resolution.",
    userPrompt: "Message: {{user_input}}",
    modelConfig: { temperature: 0.5 },
    isProduction: false,
  },
  {
    id: "v3",
    functionId: "fn-1",
    createdAt: "2026-02-04T18:00:00Z",
    label: "v10",
    systemPrompt: "You are a helpful assistant.",
    userPrompt: "{{user_input}}",
    modelConfig: {},
    isProduction: false,
  },
];

export const mockModels: ModelOption[] = [
  { id: "gpt-4o", name: "GPT-4o", provider: "OpenAI" },
  { id: "gpt-4o-mini", name: "GPT-4o Mini", provider: "OpenAI" },
  { id: "claude-3-5-sonnet", name: "Claude 3.5 Sonnet", provider: "Anthropic" },
  { id: "claude-3-haiku", name: "Claude 3 Haiku", provider: "Anthropic" },
];
