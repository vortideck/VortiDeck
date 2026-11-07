/**
 * Extism Plugin Entry Point
 *
 * Wraps our implementation functions for Extism export
 */

import { executeImpl, get_metadataImpl, registerImpl } from './main'

// Extism exports - these are the functions the host can call
export function execute(): number {
  const untypedInput = JSON.parse(Host.inputString());
  const output = executeImpl(untypedInput);
  Host.outputString(JSON.stringify(output));
  return 0;
}

export function get_metadata(): number {
  const input = Host.inputString();
  const output = get_metadataImpl(input);
  Host.outputString(JSON.stringify(output));
  return 0;
}

export function register(): number {
  const input = Host.inputString();
  const output = registerImpl(input);
  Host.outputString(output);
  return 0;
}
