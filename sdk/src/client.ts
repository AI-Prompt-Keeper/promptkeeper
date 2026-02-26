/**
 * Prompt Keeper client — drop-in style for OpenAI, pointing at the custom proxy
 * with optional fail-open to direct LLM.
 */

import type {
  ChatCompletion,
  ChatCompletionChunk,
  ChatCompletionCreateParams,
  ChatCompletionMessageParam,
  VariableValidationResult,
  Variables,
} from './types';
import { validateRequiredVariables } from './validate';

export interface PromptKeeperConfig {
  /** Base URL of the Prompt Keeper proxy (e.g. "https://proxy.example.com"). */
  baseUrl: string;
  /** Optional API key for proxy authentication (e.g. Bearer). */
  apiKey?: string;
  /** If true, on proxy failure/timeout call the LLM directly using localApiKey. */
  failOpen?: boolean;
  /** API key for direct LLM calls when failOpen is true (e.g. OpenAI key). */
  localApiKey?: string;
  /** Base URL for direct LLM when fail-open (default: OpenAI). */
  directBaseUrl?: string;
  /** Default request timeout in ms. */
  timeout?: number;
}

const DEFAULT_TIMEOUT_MS = 30_000;
const DEFAULT_DIRECT_BASE = 'https://api.openai.com';

export class PromptKeeperClient {
  private readonly baseUrl: string;
  private readonly apiKey?: string;
  private readonly failOpen: boolean;
  private readonly localApiKey?: string;
  private readonly directBaseUrl: string;
  private readonly defaultTimeout: number;

  constructor(config: PromptKeeperConfig) {
    this.baseUrl = config.baseUrl.replace(/\/$/, '');
    this.apiKey = config.apiKey;
    this.failOpen = config.failOpen ?? false;
    this.localApiKey = config.localApiKey;
    this.directBaseUrl = config.directBaseUrl ?? DEFAULT_DIRECT_BASE;
    this.defaultTimeout = config.timeout ?? DEFAULT_TIMEOUT_MS;
  }

  /**
   * Chat completions API — mirrors OpenAI's .chat.completions.create().
   * Calls the proxy with function_id + variables; on proxy failure and failOpen,
   * calls the LLM directly with model + messages using localApiKey.
   */
  get chat() {
    const self = this;
    return {
      completions: {
        create: async (
          params: ChatCompletionCreateParams
        ): Promise<ChatCompletion | AsyncIterable<ChatCompletionChunk>> => {
          const timeout = params.timeout ?? self.defaultTimeout;
          const stream = params.stream ?? false;

          try {
            return await self.callProxy(params, timeout, stream);
          } catch (proxyError) {
            if (self.failOpen && self.localApiKey && params.model && params.messages?.length) {
              return await self.callDirect(params, timeout, stream);
            }
            throw proxyError;
          }
        },
      },
    };
  }

  /**
   * Validate that the required variables for a function_id are present before calling.
   * Use with validateRequiredVariables(requiredKeys, variables) or register required keys.
   */
  static validateVariables(
    requiredKeys: string[],
    variables: Variables
  ): VariableValidationResult {
    return validateRequiredVariables(requiredKeys, variables);
  }

  /** Instance helper: validate variables for a given set of required keys. */
  validateVariables(requiredKeys: string[], variables: Variables): VariableValidationResult {
    return validateRequiredVariables(requiredKeys, variables);
  }

  private async callProxy(
    params: ChatCompletionCreateParams,
    timeout: number,
    stream: boolean
  ): Promise<ChatCompletion | AsyncIterable<ChatCompletionChunk>> {
    const url = `${this.baseUrl}/v1/execute`;
    const body = {
      function_id: params.function_id,
      variables: params.variables ?? {},
    };

    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
    };
    if (this.apiKey) {
      headers['Authorization'] = `Bearer ${this.apiKey}`;
    }

    const controller = new AbortController();
    const id = setTimeout(() => controller.abort(), timeout);

    const res = await fetch(url, {
      method: 'POST',
      headers,
      body: JSON.stringify(body),
      signal: controller.signal,
    });
    clearTimeout(id);

    if (!res.ok) {
      const text = await res.text();
      throw new Error(`Proxy error ${res.status}: ${text || res.statusText}`);
    }

    const contentType = res.headers.get('content-type') || '';
    if (!contentType.includes('text/event-stream')) {
      const text = await res.text();
      throw new Error(`Proxy did not return SSE: ${text.slice(0, 200)}`);
    }

    if (stream) {
      return this.parseSSEStream(res);
    }
    return this.parseSSEToCompletion(res);
  }

  private async parseSSEToCompletion(res: Response): Promise<ChatCompletion> {
    const reader = res.body?.getReader();
    if (!reader) throw new Error('No response body');

    const decoder = new TextDecoder();
    let buffer = '';
    let content = '';
    let id = `chatcmpl-${Date.now()}`;
    let created = Math.floor(Date.now() / 1000);

    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      buffer += decoder.decode(value, { stream: true });
      const lines = buffer.split('\n');
      buffer = lines.pop() ?? '';

      for (const line of lines) {
        if (line.startsWith('data: ')) {
          const data = line.slice(6).trim();
          if (data === '[DONE]') continue;
          try {
            const parsed = JSON.parse(data) as { error?: string; choices?: unknown[]; id?: string; created?: number };
            if (parsed.error) throw new Error(parsed.error);
            if (parsed.id) id = parsed.id;
            if (parsed.created != null) created = parsed.created;
            const choice = Array.isArray(parsed.choices) ? parsed.choices[0] : null;
            const obj = choice as { delta?: { content?: string }; message?: { content?: string } } | null;
            if (obj?.delta?.content != null) content += obj.delta.content;
            if (obj?.message?.content != null) content = obj.message.content;
          } catch (e) {
            if (e instanceof SyntaxError) continue;
            throw e;
          }
        }
      }
    }

    return {
      id,
      object: 'chat.completion',
      created,
      model: 'promptkeeper',
      choices: [
        {
          index: 0,
          message: { role: 'assistant', content },
          finish_reason: 'stop',
        },
      ],
    };
  }

  private async *parseSSEStream(res: Response): AsyncIterable<ChatCompletionChunk> {
    const reader = res.body?.getReader();
    if (!reader) throw new Error('No response body');

    const decoder = new TextDecoder();
    let buffer = '';
    let id = `chatcmpl-${Date.now()}`;
    let created = Math.floor(Date.now() / 1000);

    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      buffer += decoder.decode(value, { stream: true });
      const lines = buffer.split('\n');
      buffer = lines.pop() ?? '';

      for (const line of lines) {
        if (line.startsWith('data: ')) {
          const data = line.slice(6).trim();
          if (data === '[DONE]') continue;
          try {
            const parsed = JSON.parse(data) as { error?: string; choices?: unknown[]; id?: string; created?: number };
            if (parsed.error) throw new Error(parsed.error);
            if (parsed.id) id = parsed.id;
            if (parsed.created != null) created = parsed.created;
            yield {
              id,
              object: 'chat.completion.chunk',
              created,
              model: 'promptkeeper',
              choices: Array.isArray(parsed.choices)
                ? (parsed.choices as ChatCompletionChunk['choices'])
                : [{ index: 0, delta: {}, finish_reason: null }],
            };
          } catch (e) {
            if (e instanceof SyntaxError) continue;
            throw e;
          }
        }
      }
    }
  }

  private async callDirect(
    params: ChatCompletionCreateParams,
    timeout: number,
    stream: boolean
  ): Promise<ChatCompletion | AsyncIterable<ChatCompletionChunk>> {
    const url = `${this.directBaseUrl.replace(/\/$/, '')}/v1/chat/completions`;
    const body = {
      model: params.model,
      messages: params.messages,
      stream,
    };

    const controller = new AbortController();
    const id = setTimeout(() => controller.abort(), timeout);

    const res = await fetch(url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${this.localApiKey}`,
      },
      body: JSON.stringify(body),
      signal: controller.signal,
    });
    clearTimeout(id);

    if (!res.ok) {
      const text = await res.text();
      throw new Error(`Direct LLM error ${res.status}: ${text || res.statusText}`);
    }

    if (stream) {
      return this.parseSSEStream(res);
    }
    const data = (await res.json()) as ChatCompletion;
    return data;
  }
}
