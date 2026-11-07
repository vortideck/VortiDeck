import * as main from "./main"

import {
      ExecutionParams,
      ExecutionResult,
      PluginMetadata,
  } from './pdk'


export function execute(): number {
    const untypedInput = JSON.parse(Host.inputString())
    const input = untypedInput as ExecutionParams
      
    const output = main.executeImpl(input)
  
    Host.outputString(JSON.stringify(output))
            
  return 0
}

export function get_metadata(): number {
    const input = Host.inputString() 
    
    const output = main.get_metadataImpl(input)
  
    Host.outputString(JSON.stringify(output))
            
  return 0
}

export function register(): number {
    const input = Host.inputString() 
    
    const output = main.registerImpl(input)
  
    Host.outputString(output)
  
  return 0
}



