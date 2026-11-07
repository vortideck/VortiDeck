export type InvocationScalar = string | number | boolean | null;
export type InputTarget =
  | { type: 'httpBody'; pointer: string }
  | { type: 'httpQuery'; name: string }
  | { type: 'httpPath'; segment: number }
  | { type: 'integrationParameter'; pointer: string }
  | { type: 'systemParameter'; name: string };

export interface ActionInputContract {
  version: 1;
  inputSchema: Record<string, unknown> & {
    type: 'object';
    properties: Record<string, Record<string, unknown>>;
    required?: string[];
  };
  bindings: Array<{ input: string; target: InputTarget }>;
}

export interface InputContractSnapshot {
  revision: number;
  contract: ActionInputContract | null;
}

export type CommandInputSource =
  | { source: 'args' | 'argsNumber' | 'argsBoolean' | 'message' | 'username' | 'displayName' | 'platform' }
  | { source: 'literal'; value: InvocationScalar };
