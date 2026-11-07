# 🤖 AI Node Configuration Guide

Master VortiDeck's AI nodes to create intelligent workflows with per-node model selection and behavior customization.

## 🎯 Overview

AI nodes are the cognitive core of VortiDeck workflows. They provide:

- **🧠 Intelligent Decision Making**: Analyze data and make contextual decisions
- **🔧 Tool Integration**: Execute tools and APIs based on AI reasoning
- **📝 Content Generation**: Create text, summaries, and responses
- **🎛️ Per-Node Model Selection**: Choose different AI models for each node

## 🤖 AI Node Types

### 🤖 AI Agent (Free)
**Behavior**: FreeAgent - Autonomous AI with full tool access

**Capabilities**:
- Full access to all connected tools
- Autonomous decision-making
- Complex multi-step reasoning
- Dynamic tool selection based on context

**Best For**:
- Complex automation tasks
- Data analysis and processing
- Multi-tool workflows
- Adaptive problem-solving

**Example Use Cases**:
```
"Analyze the uploaded CSV file, identify trends, and create a summary report"
"Monitor system logs and take corrective actions if errors are found"
"Process customer emails and route them to appropriate departments"
```

### 🧠 AI Orchestrator
**Behavior**: Orchestrator - Coordinates multiple tools and workflows

**Capabilities**:
- Workflow coordination and management
- Multi-step process planning
- Conditional branching logic
- Tool sequencing and dependency management

**Best For**:
- Complex multi-stage workflows
- Process automation
- Conditional logic implementation
- Resource coordination

**Example Use Cases**:
```
"Coordinate a content publishing workflow: write, review, approve, and publish"
"Manage a customer onboarding process with multiple validation steps"
"Orchestrate data pipeline: extract, transform, validate, and load"
```

### 📝 AI Text Generator
**Behavior**: TextOnly - Pure text generation without tool access

**Capabilities**:
- High-quality text generation
- Content creation and editing
- Language translation
- Text summarization and analysis

**Best For**:
- Content creation
- Writing assistance
- Text processing
- Simple Q&A responses

**Example Use Cases**:
```
"Write a professional email response to this customer inquiry"
"Create a blog post about renewable energy trends"
"Summarize this technical document in simple terms"
```

## ⚙️ AI Model Selection

### Available Models

#### **GPT-3.5 Turbo**
- **Speed**: Very Fast ⚡⚡⚡
- **Cost**: Low 💰
- **Capability**: Good for most tasks
- **Best For**: Simple automation, basic text generation, quick responses

#### **GPT-4**
- **Speed**: Moderate ⚡⚡
- **Cost**: Medium 💰💰
- **Capability**: Advanced reasoning and complex tasks
- **Best For**: Complex analysis, detailed content creation, nuanced decision-making

#### **GPT-4 Turbo**
- **Speed**: Fast ⚡⚡⚡
- **Cost**: Medium-High 💰💰💰
- **Capability**: Latest features with enhanced performance
- **Best For**: Cutting-edge capabilities, large context tasks, advanced reasoning

### Model Selection Strategy

#### **Choose GPT-3.5 Turbo When**:
- Simple text generation or basic automation
- High-frequency, low-complexity tasks
- Cost optimization is important
- Speed is critical

#### **Choose GPT-4 When**:
- Complex reasoning or analysis required
- High-quality content generation needed
- Nuanced decision-making is important
- Working with complex data structures

#### **Choose GPT-4 Turbo When**:
- Need latest AI capabilities
- Working with large amounts of context
- Require best possible output quality
- Performance and capability are priorities

## 🔧 Configuration Guide

### Basic Configuration

1. **Add AI Node**: Drag from Node Library to canvas
2. **Select Model**: Click the model selector (⚙️) button
3. **Choose Model**: Select from available AI models
4. **Configure Inputs**: Set up the required inputs

### Input Configuration

#### **User Message** (Required)
- **Purpose**: Primary instruction or prompt for the AI
- **Tips**: 
  - Be specific and clear
  - Include context and expected output format
  - Use examples when helpful

```
Good: "Analyze the sales data and identify the top 3 performing products with their revenue figures"
Better: "Analyze the sales data in the attached CSV. Identify the top 3 performing products by revenue and format the response as: Product Name: $Revenue"
```

#### **Context** (Optional)
- **Purpose**: Additional context and background information
- **Tips**:
  - Include relevant background information
  - Reference previous workflow outputs
  - Provide domain-specific knowledge

#### **Previous Outputs** (Automatic)
- **Purpose**: Results from previous nodes in the workflow
- **Usage**: Automatically populated when connected to other nodes

### Advanced Configuration

#### **Tool Access Configuration**
For FreeAgent and Orchestrator behaviors:

1. **Connect Tools**: Link tool nodes to AI node inputs
2. **Tool Descriptions**: AI automatically receives tool descriptions
3. **Access Control**: AI decides which tools to use based on context

#### **Flow Context**
- **Current Node ID**: Automatically set
- **Next Node Types**: Inferred from connections
- **Expected Outputs**: Based on connected downstream nodes

## 🔗 Connection Patterns

### Basic AI Flow
```
Trigger → AI Node → Output Tool
```

### Multi-Tool AI Flow
```
Trigger → AI Node → Multiple Tools
                 ↓
              Aggregator → Final Output
```

### Conditional AI Flow
```
Trigger → AI Node → Branch Condition
                 ↓              ↓
            Tool A          Tool B
```

### Chained AI Flow
```
Trigger → AI Node 1 → AI Node 2 → AI Node 3 → Output
```

## 💡 Best Practices

### Prompt Engineering

#### **Be Specific**
```
❌ "Process this data"
✅ "Analyze the sales data and calculate the month-over-month growth rate"
```

#### **Provide Context**
```
❌ "Write an email"
✅ "Write a professional follow-up email to a potential client who attended our product demo yesterday"
```

#### **Define Output Format**
```
❌ "Summarize the report"
✅ "Summarize the report in 3 bullet points, each under 50 words"
```

### Model Selection

#### **Match Complexity to Capability**
- Simple tasks → GPT-3.5 Turbo
- Complex analysis → GPT-4
- Latest features → GPT-4 Turbo

#### **Consider Cost vs. Quality**
- High-frequency automation → GPT-3.5 Turbo
- Critical decision-making → GPT-4
- Premium features → GPT-4 Turbo

#### **Optimize for Speed When Needed**
- Real-time responses → GPT-3.5 Turbo or GPT-4 Turbo
- Batch processing → Any model based on complexity

### Error Handling

#### **Validate Inputs**
- Check that required inputs are connected
- Verify data format and content
- Handle missing or invalid data gracefully

#### **Monitor Execution**
- Watch for error status indicators
- Review AI reasoning in debug output
- Check tool execution results

#### **Implement Fallbacks**
- Use branch conditions for error handling
- Provide alternative execution paths
- Include retry logic for transient failures

## 🐛 Troubleshooting

### Common Issues

#### **AI Node Not Executing**
- ✅ Verify AI model is selected
- ✅ Check that flow input is connected
- ✅ Ensure user message is provided
- ✅ Validate API key configuration

#### **Poor AI Responses**
- ✅ Improve prompt clarity and specificity
- ✅ Add more context and examples
- ✅ Consider upgrading to a more capable model
- ✅ Review input data quality

#### **Tool Access Issues**
- ✅ Verify tools are properly connected
- ✅ Check tool descriptions and parameters
- ✅ Ensure AI behavior supports tool access
- ✅ Review tool execution permissions

#### **Performance Problems**
- ✅ Optimize prompt length
- ✅ Choose appropriate model for task complexity
- ✅ Reduce unnecessary context
- ✅ Consider parallel execution for independent tasks

## 🔗 Next Steps

- **[Flow Builder Guide](flow-builder.md)**: Learn to create complex workflows
- **Tool Integration**: Connect external services
- **API Documentation**: Integrate with other applications

---

Ready to harness the power of AI in your workflows? Start configuring your AI nodes! 🤖 