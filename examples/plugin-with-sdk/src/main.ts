/**
 * Multi-Node Example Plugin using @vortideck/plugin-sdk
 *
 * Demonstrates:
 * - Multiple nodes in a single plugin
 * - NodeContext with progress events
 * - Plugin-scoped storage API
 * - Clean defineNode helper
 */

import { createPlugin, defineNode, FlowParams } from '@vortideck/plugin-sdk'
import type { NodeContext } from '@vortideck/plugin-sdk'
import vortideck from '@vortideck/plugin-sdk'

// ✨ Define multiple nodes in a single plugin!
// Note: Plugin metadata (id, name, version, author) comes from manifest.json
export const { execute, register, get_metadata } = createPlugin({
  // Multiple nodes in one plugin!
  nodes: {
    // Node 1: Text to Uppercase with progress tracking and MULTIPLE outputs
    uppercase: defineNode({
      metadata: {
        id: "example:text-uppercase",
        name: "Text to Uppercase",
        description: "Convert text to uppercase with progress tracking",
        category: "text",
        type_color: "#4A90E2",
        // Define multiple output ports - each property becomes a separate output
        output_schema: {
          properties: {
            uppercase: {
              type: "string",
              description: "The uppercased text"
            },
            length: {
              type: "number",
              description: "Length of the text"
            },
            original: {
              type: "string",
              description: "Original input text"
            }
          }
        },
        parameters_schema: {
          type: "object",
          properties: {
            input: {
              type: "string",
              title: "Input Text",
              default: "Hello World"
            }
          }
        }
      },

      execute(input: any, context: NodeContext) {
        try {
          vortideck.log("Uppercase Node: Starting execution")

          // Emit progress at start
          context.emitProgress(0.0, "Starting uppercase conversion")

          const params = new FlowParams(input)
          const inputText = params.getInput("Hello World")

          // Simulate processing with progress updates
          context.emitProgress(0.5, "Processing text")
          const processedText = inputText.toUpperCase()

          // Emit completion progress
          context.emitProgress(1.0, "Completed")

          // Emit custom event
          context.emitEvent("text_processed", {
            original_length: inputText.length,
            processed_length: processedText.length
          })

          return {
            success: true,
            output: processedText,
            outputs: [processedText]
          }
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : String(error)
          return {
            success: false,
            error: errorMessage,
            error_code: "UPPERCASE_ERROR"
          }
        }
      }
    }),

    // Node 2: Text to Lowercase with storage
    lowercase: defineNode({
      metadata: {
        id: "example:text-lowercase",
        name: "Text to Lowercase",
        description: "Convert text to lowercase with execution counting",
        category: "text",
        type_color: "#E24A90",
        // Optional: Can omit output_schema for single default output
        parameters_schema: {
          type: "object",
          properties: {
            input: {
              type: "string",
              title: "Input Text",
              default: "HELLO WORLD"
            }
          }
        }
      },

      execute(input: any, context: NodeContext) {
        try {
          vortideck.log("Lowercase Node: Starting execution")

          context.emitProgress(0.0, "Starting lowercase conversion")

          const params = new FlowParams(input)
          const inputText = params.getInput("HELLO WORLD")

          // Use storage to track execution count
          let count = 0
          try {
            const storedCount = context.getStorage("execution_count")
            count = parseInt(storedCount) || 0
          } catch (e) {
            vortideck.log("No previous execution count found (storage not yet implemented)")
          }

          count++

          try {
            context.setStorage("execution_count", count.toString())
            vortideck.log(`Execution count: ${count}`)
          } catch (e) {
            vortideck.log("Could not save execution count (storage not yet implemented)")
          }

          context.emitProgress(0.5, `Processing (execution #${count})`)
          const processedText = inputText.toLowerCase()

          context.emitProgress(1.0, "Completed")

          return {
            success: true,
            output: processedText,
            outputs: [processedText]
          }
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : String(error)
          return {
            success: false,
            error: errorMessage,
            error_code: "LOWERCASE_ERROR"
          }
        }
      }
    }),

    // Node 3: Text Reverser
    reverse: defineNode({
      metadata: {
        id: "example:text-reverse",
        name: "Reverse Text",
        description: "Reverse the input text",
        category: "text",
        type_color: "#90E24A",
        parameters_schema: {
          type: "object",
          properties: {
            input: {
              type: "string",
              title: "Input Text",
              default: "Hello"
            }
          }
        }
      },

      execute(input: any, context: NodeContext) {
        try {
          vortideck.log("Reverse Node: Starting execution")
          context.emitProgress(0.3, "Reversing text")

          const params = new FlowParams(input)
          const inputText = params.getInput("Hello")
          const reversed = inputText.split('').reverse().join('')

          context.emitProgress(1.0, "Completed")

          return {
            success: true,
            output: reversed,
            outputs: [reversed]
          }
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : String(error)
          return {
            success: false,
            error: errorMessage,
            error_code: "REVERSE_ERROR"
          }
        }
      }
    }),

    // Node 4: Title Case
    titlecase: defineNode({
      metadata: {
        id: "example:text-titlecase",
        name: "Title Case",
        description: "Convert text to title case (capitalize first letter of each word)",
        category: "text",
        type_color: "#E2904A",
        parameters_schema: {
          type: "object",
          properties: {
            input: {
              type: "string",
              title: "Input Text",
              default: "hello world"
            }
          }
        }
      },

      execute(input: any, context: NodeContext) {
        try {
          vortideck.log("Title Case Node: Starting execution")
          context.emitProgress(0.3, "Converting to title case")

          const params = new FlowParams(input)
          const inputText = params.getInput("hello world")

          const titleCased = inputText
            .toLowerCase()
            .split(' ')
            .map(word => word.charAt(0).toUpperCase() + word.slice(1))
            .join(' ')

          context.emitProgress(1.0, "Completed")

          return {
            success: true,
            output: titleCased,
            outputs: [titleCased]
          }
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : String(error)
          return {
            success: false,
            error: errorMessage,
            error_code: "TITLECASE_ERROR"
          }
        }
      }
    }),

    // Node 5: Capitalize First Letter
    capitalize: defineNode({
      metadata: {
        id: "example:text-capitalize",
        name: "Capitalize",
        description: "Capitalize only the first letter of the text",
        category: "text",
        type_color: "#4AE290",
        parameters_schema: {
          type: "object",
          properties: {
            input: {
              type: "string",
              title: "Input Text",
              default: "hello world"
            }
          }
        }
      },

      execute(input: any, context: NodeContext) {
        try {
          vortideck.log("Capitalize Node: Starting execution")
          context.emitProgress(0.3, "Capitalizing first letter")

          const params = new FlowParams(input)
          const inputText = params.getInput("hello world")

          const capitalized = inputText.charAt(0).toUpperCase() + inputText.slice(1).toLowerCase()

          context.emitProgress(1.0, "Completed")

          return {
            success: true,
            output: capitalized,
            outputs: [capitalized]
          }
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : String(error)
          return {
            success: false,
            error: errorMessage,
            error_code: "CAPITALIZE_ERROR"
          }
        }
      }
    }),

    // Node 6: Trim Whitespace
    trim: defineNode({
      metadata: {
        id: "example:text-trim",
        name: "Trim Whitespace",
        description: "Remove leading and trailing whitespace",
        category: "text",
        type_color: "#904AE2",
        parameters_schema: {
          type: "object",
          properties: {
            input: {
              type: "string",
              title: "Input Text",
              default: "  hello world  "
            }
          }
        }
      },

      execute(input: any, context: NodeContext) {
        try {
          vortideck.log("Trim Node: Starting execution")
          context.emitProgress(0.3, "Trimming whitespace")

          const params = new FlowParams(input)
          const inputText = params.getInput("  hello world  ")

          const trimmed = inputText.trim()

          context.emitProgress(1.0, "Completed")

          return {
            success: true,
            output: trimmed,
            outputs: [trimmed]
          }
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : String(error)
          return {
            success: false,
            error: errorMessage,
            error_code: "TRIM_ERROR"
          }
        }
      }
    }),

    // Node 7: Word Count with multiple outputs
    wordcount: defineNode({
      metadata: {
        id: "example:text-wordcount",
        name: "Word Count",
        description: "Count words, characters, and lines in text",
        category: "text",
        type_color: "#E2C44A",
        output_schema: {
          properties: {
            word_count: {
              type: "number",
              description: "Number of words"
            },
            character_count: {
              type: "number",
              description: "Number of characters"
            },
            line_count: {
              type: "number",
              description: "Number of lines"
            },
            original: {
              type: "string",
              description: "Original text"
            }
          }
        },
        parameters_schema: {
          type: "object",
          properties: {
            input: {
              type: "string",
              title: "Input Text",
              default: "Hello world"
            }
          }
        }
      },

      execute(input: any, context: NodeContext) {
        try {
          vortideck.log("Word Count Node: Starting execution")
          context.emitProgress(0.3, "Counting words and characters")

          const params = new FlowParams(input)
          const inputText = params.getInput("Hello world")

          const wordCount = inputText.trim().split(/\s+/).filter(w => w.length > 0).length
          const charCount = inputText.length
          const lineCount = inputText.split('\n').length

          context.emitProgress(1.0, "Completed")

          context.emitEvent("word_count_result", {
            words: wordCount,
            chars: charCount,
            lines: lineCount
          })

          return {
            success: true,
            output: `Words: ${wordCount}, Characters: ${charCount}, Lines: ${lineCount}`,
            outputs: [wordCount, charCount, lineCount, inputText]
          }
        } catch (error) {
          const errorMessage = error instanceof Error ? error.message : String(error)
          return {
            success: false,
            error: errorMessage,
            error_code: "WORDCOUNT_ERROR"
          }
        }
      }
    })
  },

  // Optional: Plugin-level initialization
  onPluginRegister() {
    vortideck.log("🎉 Text Processor Suite registered with 7 nodes!")
  }
})
