# @promptkeeper/sdk

TypeScript SDK for the Prompt Keeper proxy. Drop-in style for the OpenAI client: use `.chat.completions.create()` with `function_id` and `variables`, with optional fail-open to the LLM and variable validation.

## Install

```bash
npm install @promptkeeper/sdk
```

## Usage

### Basic (proxy only)

```ts
import { PromptKeeperClient } from '@promptkeeper/sdk';

const client = new PromptKeeperClient({
  baseUrl: 'https://your-proxy.example.com',
  apiKey: process.env.PROMPTKEEPER_API_KEY, // optional
});

const completion = await client.chat.completions.create({
  function_id: 'customer_support_reply',
  variables: { name: 'Alice', query: 'How do I reset my password?' },
});

console.log(completion.choices[0].message.content);
```

### With fail-open (proxy first, then direct LLM)

If the proxy fails or times out, the SDK can call the LLM directly using your local API key:

```ts
const client = new PromptKeeperClient({
  baseUrl: 'https://your-proxy.example.com',
  failOpen: true,
  localApiKey: process.env.OPENAI_API_KEY,
  directBaseUrl: 'https://api.openai.com', // optional, default
});

const completion = await client.chat.completions.create({
  function_id: 'customer_support_reply',
  variables: { name: 'Alice', query: 'Reset password?' },
  model: 'gpt-4o',
  messages: [
    { role: 'system', content: 'You are a helpful assistant.' },
    { role: 'user', content: 'How do I reset my password?' },
  ],
});
```

When the proxy fails, the SDK uses `model` and `messages` for the direct call. Always provide them if you use `failOpen: true`.

### Variable safety (validate before calling)

Validate that required template variables are present before making the request:

```ts
import { PromptKeeperClient, validateRequiredVariables } from '@promptkeeper/sdk';

const requiredKeys = ['name', 'query'];
const variables = { name: 'Alice', query: 'Hello' };

const result = validateRequiredVariables(requiredKeys, variables);
if (!result.valid) {
  throw new Error(`Missing variables: ${result.missing.join(', ')}`);
}

const client = new PromptKeeperClient({ baseUrl: 'https://proxy.example.com' });
const completion = await client.chat.completions.create({
  function_id: 'my_function',
  variables,
});
```

Or use the instance helper:

```ts
const result = client.validateVariables(requiredKeys, variables);
if (!result.valid) throw new Error(`Missing: ${result.missing.join(', ')}`);
```

### Streaming

```ts
const stream = await client.chat.completions.create({
  function_id: 'my_function',
  variables: { topic: 'TypeScript' },
  stream: true,
});

for await (const chunk of stream) {
  const text = chunk.choices[0]?.delta?.content;
  if (text) process.stdout.write(text);
}
```

## API

- **`new PromptKeeperClient(config)`**  
  - `config.baseUrl` — Proxy base URL (e.g. `https://proxy.example.com`).  
  - `config.apiKey` — Optional proxy auth (Bearer).  
  - `config.failOpen` — If true, on proxy failure call LLM directly.  
  - `config.localApiKey` — API key for direct LLM when fail-open.  
  - `config.directBaseUrl` — Base URL for direct LLM (default `https://api.openai.com`).  
  - `config.timeout` — Default request timeout in ms.

- **`client.chat.completions.create(params)`**  
  - `params.function_id` — Function identifier for the proxy.  
  - `params.variables` — Optional template variables.  
  - `params.stream` — Optional; if true, returns an async iterable of chunks.  
  - `params.model`, `params.messages` — Used for direct call when fail-open.  
  - `params.timeout` — Override timeout for this request.

- **`validateRequiredVariables(requiredKeys, variables)`**  
  Returns `{ valid: boolean, missing: string[] }`.

- **`PromptKeeperClient.validateVariables(requiredKeys, variables)`**  
  Static helper; same as above.
