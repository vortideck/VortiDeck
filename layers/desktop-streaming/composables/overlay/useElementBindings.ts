/**
 * Element-Function Binding System
 * Maps function outputs to specific parts of element structure
 */

import { ref, computed, watch } from 'vue'

export interface ElementBinding {
  [key: string]: string | FunctionExpression
}

export interface FunctionExpression {
  expression: string
  functions: string[]
}

export interface DynamicElement {
  id: string
  type?: string
  template: string
  bindings: ElementBinding
  dynamicStyles?: ElementBinding
  style?: Record<string, any>
  x: number
  y: number
  width: number
  height: number
  visible?: boolean | string
}

export const useElementBindings = () => {

  /**
   * Parse binding expression to extract function references
   */
  const parseExpression = (expression: string): FunctionExpression => {
    // Find all function references like {functionName.property}
    const functionRefs = expression.match(/\{([^}]+)\}/g) || []
    const functions = new Set<string>()

    functionRefs.forEach(ref => {
      const cleanRef = ref.slice(1, -1) // Remove { }
      const functionName = cleanRef.split('.')[0]
      functions.add(functionName)
    })

    return {
      expression,
      functions: Array.from(functions)
    }
  }

  /**
   * Evaluate expression with function data
   */
  const evaluateExpression = (expression: string, functionData: Record<string, any>): any => {
    let result = expression

    // Replace function references with actual values
    const functionRefs = expression.match(/\{([^}]+)\}/g) || []

    for (const ref of functionRefs) {
      const path = ref.slice(1, -1) // Remove { }
      const value = getNestedValue(functionData, path)
      result = result.replace(ref, String(value ?? ''))
    }

    // Handle conditional expressions (simple ones)
    if (result.includes(' ? ') && result.includes(' : ')) {
      try {
        // For safety, only allow simple comparisons
        const safeResult = result.replace(/[^a-zA-Z0-9\s><=!?:'".,_-]/g, '')
        return Function(`"use strict"; return (${safeResult})`)()
      } catch {
        return result
      }
    }

    // Handle numeric operations
    if (/^[\d\s+\-*/.()]+$/.test(result)) {
      try {
        return Function(`"use strict"; return (${result})`)()
      } catch {
        return result
      }
    }

    return result
  }

  /**
   * Get nested object value by path (e.g., "user.profile.name")
   */
  const getNestedValue = (obj: any, path: string): any => {
    return path.split('.').reduce((current, key) => {
      return current?.[key]
    }, obj)
  }

  /**
   * Process element template with function data
   */
  const processTemplate = (template: string, functionData: Record<string, any>): string => {
    let processedTemplate = template

    console.log('🔧 PROCESSING TEMPLATE with data:', functionData)

    // Find all {something} references in the template
    const templateRefs = template.match(/\{([^}]+)\}/g) || []

    console.log('🔧 Found template refs:', templateRefs)

    for (const ref of templateRefs) {
      const path = ref.slice(1, -1) // Remove { }
      console.log('🔧 Processing ref:', ref, 'path:', path)

      // Get the value using the nested path helper
      const value = getNestedValue(functionData, path)
      console.log('🔧 Value found:', value)

      if (value !== undefined) {
        processedTemplate = processedTemplate.replace(ref, String(value))
        console.log('🔧 Replaced', ref, 'with', value)
      }
    }

    console.log('🔧 Final processed template preview:', processedTemplate.substring(0, 300))
    return processedTemplate
  }

  /**
   * Process dynamic styles with function data
   */
  const processDynamicStyles = (
    dynamicStyles: ElementBinding,
    functionData: Record<string, any>
  ): Record<string, any> => {
    const processedStyles: Record<string, any> = {}

    Object.entries(dynamicStyles).forEach(([styleProperty, expression]) => {
      if (typeof expression === 'string') {
        processedStyles[styleProperty] = evaluateExpression(expression, functionData)
      }
    })

    return processedStyles
  }

  /**
   * Get all function dependencies for an element
   */
  const getElementFunctionDependencies = (element: DynamicElement): string[] => {
    const functions = new Set<string>()

    // From template
    const templateExpr = parseExpression(element.template)
    templateExpr.functions.forEach(f => functions.add(f))

    // From bindings
    Object.values(element.bindings).forEach(binding => {
      if (typeof binding === 'string') {
        const expr = parseExpression(binding)
        expr.functions.forEach(f => functions.add(f))
      }
    })

    // From dynamic styles
    if (element.dynamicStyles) {
      Object.values(element.dynamicStyles).forEach(binding => {
        if (typeof binding === 'string') {
          const expr = parseExpression(binding)
          expr.functions.forEach(f => functions.add(f))
        }
      })
    }

    // From visibility condition
    if (typeof element.visible === 'string') {
      const expr = parseExpression(element.visible)
      expr.functions.forEach(f => functions.add(f))
    }

    return Array.from(functions)
  }

  /**
   * Render element with function data applied
   */
  const renderElement = (
    element: DynamicElement,
    functionData: Record<string, any>
  ): {
    html: string
    styles: Record<string, any>
    visible: boolean
  } => {
    // Process template
    const html = processTemplate(element.template, functionData)

    // Process styles
    const dynamicStyles = element.dynamicStyles
      ? processDynamicStyles(element.dynamicStyles, functionData)
      : {}

    const styles = {
      ...element.style,
      ...dynamicStyles
    }

    // Process visibility
    let visible = true
    if (typeof element.visible === 'string') {
      visible = Boolean(evaluateExpression(element.visible, functionData))
    } else if (typeof element.visible === 'boolean') {
      visible = element.visible
    }

    return { html, styles, visible }
  }

  /**
   * Create element template builder helpers
   */
  const createTemplateHelpers = () => {
    return {
      // Counter template
      counter: (valueBinding: string, labelBinding?: string) => `
        <div class="counter-widget">
          <div class="counter-value">{${valueBinding}}</div>
          ${labelBinding ? `<div class="counter-label">{${labelBinding}}</div>` : ''}
        </div>
      `,

      // Progress bar template
      progressBar: (currentBinding: string, targetBinding: string, labelBinding?: string) => `
        <div class="progress-widget">
          ${labelBinding ? `<div class="progress-label">{${labelBinding}}</div>` : ''}
          <div class="progress-bar">
            <div class="progress-fill" style="width: {${currentBinding}} / {${targetBinding}} * 100%"></div>
          </div>
          <div class="progress-text">{${currentBinding}} / {${targetBinding}}</div>
        </div>
      `,

      // Text template
      text: (contentBinding: string) => `
        <div class="text-widget">{${contentBinding}}</div>
      `,

      // Custom template
      custom: (html: string) => html
    }
  }

  return {
    parseExpression,
    evaluateExpression,
    processTemplate,
    processDynamicStyles,
    getElementFunctionDependencies,
    renderElement,
    createTemplateHelpers
  }
}