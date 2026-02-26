/**
 * Types for Prompt Keeper SDK — aligned with proxy and OpenAI-style usage.
 */

/** Variables map for prompt templating (function_id templates). */
export type Variables = Record<string, unknown>;

/** OpenAI-style message for direct (fail-open) calls. */
export interface ChatCompletionMessageParam {
  role: 'system' | 'user' | 'assistant';
  content: string;
}

/**
 * Request params for chat.completions.create().
 * Use function_id + variables for proxy; model + messages for direct (fail-open) call.
 */
export interface ChatCompletionCreateParams {
  /** Function identifier for the proxy (required for proxy path). */
  function_id: string;
  /** Template variables for the function's prompt. */
  variables?: Variables;
  /** Stream response (SSE). Default false. */
  stream?: boolean;
  /** Model for direct LLM call when fail-open is used (e.g. "gpt-4o"). */
  model?: string;
  /** Messages for direct LLM call when fail-open is used. */
  messages?: ChatCompletionMessageParam[];
  /** Request timeout in ms. Default 30000. */
  timeout?: number;
}

/** Non-streaming response — OpenAI-compatible shape. */
export interface ChatCompletion {
  id: string;
  object: 'chat.completion';
  created: number;
  model: string;
  choices: Array<{
    index: number;
    message: { role: 'assistant'; content: string };
    finish_reason: string | null;
  }>;
  usage?: { prompt_tokens: number; completion_tokens: number; total_tokens: number };
}

/** Streaming choice delta — OpenAI-compatible. */
export interface ChatCompletionChunk {
  id: string;
  object: 'chat.completion.chunk';
  created: number;
  model: string;
  choices: Array<{
    index: number;
    delta: { role?: 'assistant'; content?: string };
    finish_reason: string | null;
  }>;
}

/** Result of validating variables for a function. */
export interface VariableValidationResult {
  valid: boolean;
  missing: string[];
}
