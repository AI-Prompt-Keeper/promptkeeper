export interface FunctionRow {
  id: string;
  name: string;
  activeVersion: string;
  avgLatencyMs: number;
  costLast24h: number;
}

export interface PromptVersion {
  id: string;
  functionId: string;
  createdAt: string;
  label?: string;
  systemPrompt: string;
  userPrompt: string;
  modelConfig: Record<string, unknown>;
  isProduction?: boolean;
}

export interface ModelOption {
  id: string;
  name: string;
  provider: string;
}
