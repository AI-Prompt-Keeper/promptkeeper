/**
 * Variable validation — ensure required variables for a function_id are present
 * before making the network call.
 */

import type { VariableValidationResult, Variables } from './types';

/**
 * Validates that all required variable keys are present (and non-undefined) in the given object.
 * Use this before calling chat.completions.create() to avoid proxy errors for missing template vars.
 *
 * @param requiredKeys - Keys that must exist in `variables` (e.g. from your function's template).
 * @param variables - The variables object you intend to send (e.g. { name: "Alice", query: "Hello" }).
 * @returns { valid: true } or { valid: false, missing: string[] }.
 *
 * @example
 * const result = validateRequiredVariables(['name', 'query'], { name: 'Alice', query: 'Hi' });
 * if (!result.valid) throw new Error(`Missing: ${result.missing.join(', ')}`);
 */
export function validateRequiredVariables(
  requiredKeys: string[],
  variables: Variables
): VariableValidationResult {
  if (!variables || typeof variables !== 'object') {
    return { valid: requiredKeys.length === 0, missing: requiredKeys.slice() };
  }
  const missing = requiredKeys.filter((key) => variables[key] === undefined);
  return {
    valid: missing.length === 0,
    missing,
  };
}
