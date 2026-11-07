/**
 * VortiDeck Plugin SDK - Clean API
 *
 * AUTO-GENERATED from Rust host functions
 * DO NOT EDIT MANUALLY
 *
 * This file provides a developer-friendly wrapper around the auto-generated PDK.
 */

import {
  vortideck_log,
  vortideck_register_node,
  vortideck_get_context,
  vortideck_emit_progress,
  vortideck_emit_event,
  vortideck_storage_get,
  vortideck_storage_set,
  vortideck_read_file,
  vortideck_write_file,
  vortideck_http_request,
} from './pdk.generated'

/**
 * log
 * @param message - string
 * @returns string
 */
export const log = vortideck_log

/**
 * registerNode
 * @param metadata - string
 * @returns boolean
 */
export const registerNode = vortideck_register_node

/**
 * getContext
 * @returns string
 */
export const getContext = vortideck_get_context

/**
 * emitProgress
 * @param progress_json - string
 * @returns boolean
 */
export const emitProgress = vortideck_emit_progress

/**
 * emitEvent
 * @param event_json - string
 * @returns boolean
 */
export const emitEvent = vortideck_emit_event

/**
 * storageGet
 * @param key - string
 * @returns string
 */
export const storageGet = vortideck_storage_get

/**
 * storageSet
 * @param key - string
 * @param value - string
 * @returns boolean
 */
export const storageSet = vortideck_storage_set

/**
 * readFile
 * @param path - string
 * @returns string
 */
export const readFile = vortideck_read_file

/**
 * writeFile
 * @param path - string
 * @param content - string
 * @returns boolean
 */
export const writeFile = vortideck_write_file

/**
 * httpRequest
 * @param options_json - string
 * @returns string
 */
export const httpRequest = vortideck_http_request

/**
 * Default export - namespace object for clean API
 *
 * @example
 * ```typescript
 * import vortideck from '@vortideck/plugin-sdk'
 *
 * vortideck.log()
 * ```
 */
const vortideck = {
  log,
  registerNode,
  getContext,
  emitProgress,
  emitEvent,
  storageGet,
  storageSet,
  readFile,
  writeFile,
  httpRequest,
}

export default vortideck
