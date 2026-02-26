/**
 * @promptkeeper/sdk
 *
 * Drop-in style client for the Prompt Keeper proxy with optional fail-open to direct LLM.
 *
 * - Use .chat.completions.create({ function_id, variables }) to call the proxy.
 * - Use validateVariables(requiredKeys, variables) to check required template vars before calling.
 * - Configure failOpen + localApiKey to fall back to direct OpenAI (or other base URL) on proxy failure.
 */

export { PromptKeeperClient } from './client';
export type { PromptKeeperConfig } from './client';
export { validateRequiredVariables } from './validate';
export type {
  ChatCompletion,
  ChatCompletionChunk,
  ChatCompletionCreateParams,
  ChatCompletionMessageParam,
  VariableValidationResult,
  Variables,
} from './types';
