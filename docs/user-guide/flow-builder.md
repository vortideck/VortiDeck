# 🎨 Flow Builder Guide

Master the VortiDeck visual flow builder to create powerful AI-driven workflows with drag-and-drop simplicity.

## 🎯 Overview

The Flow Builder is VortiDeck's visual interface for creating AI workflows. It allows you to:

- **Design** complex automation workflows visually
- **Connect** AI nodes with tools and triggers
- **Configure** per-node AI model selection
- **Execute** flows with real-time feedback

## 🧩 Core Components

### 🚀 Trigger Nodes (Start Points)

Every flow begins with a trigger that defines when the workflow should execute:

- **🌐 Webhook Trigger**: HTTP requests from external services
- **🔘 Button Trigger**: Manual execution via UI button
- **⏰ Timer Trigger**: Scheduled execution at specific times
- **📁 File System Trigger**: File changes and directory monitoring
- **📡 MQTT Trigger**: IoT device messages and sensor data

### 🤖 AI Nodes (Intelligence)

AI nodes provide the cognitive capabilities of your workflows:

#### **🤖 AI Agent (Free)**
- **Behavior**: FreeAgent - Autonomous AI with full tool access
- **Use Case**: Complex tasks requiring decision-making and tool usage
- **Example**: "Analyze this data and create a summary report"

#### **🧠 AI Orchestrator**
- **Behavior**: Orchestrator - Coordinates multiple tools and workflows
- **Use Case**: Multi-step processes with conditional branching
- **Example**: "Process customer inquiry and route to appropriate department"

#### **📝 AI Text Generator**
- **Behavior**: TextOnly - Pure text generation without tools
- **Use Case**: Content creation, writing assistance, simple responses
- **Example**: "Write a professional email response"

### ⚙️ Control Flow Nodes

Control the execution path and data flow:

- **⏱️ Delay**: Add pauses between operations
- **🔀 Branch Condition**: Conditional execution based on boolean logic
- **📝 Text Value**: Static text input for workflows

### 🛠️ Tool Nodes

Integrate with external systems and services:

- **🔧 System Tools**: File operations, calculations, system commands
- **🎵 Audio Tools**: Sound playback, volume control, text-to-speech
- **🌐 Integration Tools**: APIs, web services, cloud platforms
- **🎮 Streaming Tools**: OBS control, stream deck integration

## 🎨 Building Your First Flow

### Step 1: Plan Your Workflow

Before building, outline your workflow:

1. **What triggers it?** (Button press, webhook, timer, etc.)
2. **What should the AI do?** (Analyze, generate, decide, etc.)
3. **What tools are needed?** (APIs, files, audio, etc.)
4. **What's the expected output?** (Text, actions, notifications, etc.)

### Step 2: Add the Trigger

1. **Open** the Node Library on the left sidebar
2. **Expand** the "Start Triggers" category
3. **Drag** your chosen trigger to the canvas
4. **Configure** trigger-specific settings

### Step 3: Add AI Intelligence

1. **Expand** the "AI Tools" category
2. **Drag** an AI node to the canvas
3. **Connect** the trigger's flow output to the AI node's flow input
4. **Configure** the AI node:
   - **Select AI Model**: Click the model selector (⚙️) to choose GPT-3.5, GPT-4, etc.
   - **Set Behavior**: The behavior is pre-configured based on node type
   - **Write Prompt**: Enter your instructions in the "User Message" field

### Step 4: Add Tools and Actions

1. **Browse** tool categories in the Node Library
2. **Drag** required tools to the canvas
3. **Connect** AI outputs to tool inputs
4. **Configure** tool parameters

### Step 5: Test and Execute

1. **Click** the "Execute Flow" button
2. **Monitor** real-time execution status
3. **Review** outputs and results
4. **Debug** any issues using the Debug button

## 🔗 Connection Types

### Flow Connections (Orange)
- **Purpose**: Control execution order
- **Visual**: Thick orange lines with animated flow indicators
- **Usage**: Connect flow outputs to flow inputs

### Data Connections (Colored by Type)
- **String** (Blue): Text data
- **Number** (Red): Numeric values  
- **Boolean** (Green): True/false values
- **Object** (Purple): Complex data structures
- **Array** (Orange): Lists of items

## ⚙️ Advanced Features

### Per-Node AI Model Selection

Each AI node can use a different AI model:

1. **Click** the model selector (⚙️) on any AI node
2. **Choose** from available models:
   - **GPT-3.5 Turbo**: Fast, cost-effective for simple tasks
   - **GPT-4**: Advanced reasoning for complex tasks
   - **GPT-4 Turbo**: Latest model with enhanced capabilities
3. **Save** your selection

### Multi-Node Selection

- **Ctrl+Click**: Add/remove nodes from selection
- **Right-Click+Drag**: Select multiple nodes with rectangle
- **Delete**: Remove all selected nodes
- **Move**: Drag to move all selected nodes together

### Canvas Navigation

- **Pan**: Left-click and drag on empty canvas
- **Zoom**: Mouse wheel to zoom in/out
- **Grid**: Toggle background grid in settings
- **Snap**: Enable snap-to-grid for precise alignment

### Real-Time Execution Monitoring

During flow execution, watch for:

- **🟠 Running**: Node is currently executing
- **🟢 Completed**: Node finished successfully
- **🔴 Error**: Node encountered an error
- **⚪ Idle**: Node waiting to execute

## 💡 Best Practices

### Flow Design
1. **Keep it Simple**: Start with linear flows before adding complexity
2. **Use Descriptive Names**: Rename nodes to reflect their purpose
3. **Group Related Logic**: Use visual spacing to organize related nodes
4. **Test Incrementally**: Build and test small sections before expanding

### AI Configuration
1. **Choose Appropriate Models**: Use GPT-3.5 for simple tasks, GPT-4 for complex reasoning
2. **Write Clear Prompts**: Be specific about what you want the AI to do
3. **Provide Context**: Use previous outputs and context inputs effectively
4. **Set Expectations**: Define the expected output format in your prompts

### Performance Optimization
1. **Minimize AI Calls**: Combine multiple tasks in a single AI node when possible
2. **Use Caching**: Store frequently used data in variables
3. **Parallel Execution**: Design flows to run independent tasks simultaneously
4. **Error Handling**: Add branch conditions to handle potential failures

## 🐛 Troubleshooting

### Common Issues

**Flow Won't Execute**
- ✅ Check that all required inputs are connected
- ✅ Verify AI model is selected for AI nodes
- ✅ Ensure trigger is properly configured

**AI Node Errors**
- ✅ Check AI model availability and API keys
- ✅ Review prompt clarity and specificity
- ✅ Verify input data format and content

**Connection Problems**
- ✅ Ensure compatible data types (string to string, etc.)
- ✅ Check that outputs connect to inputs (not input to input)
- ✅ Verify flow connections use flow ports

**Performance Issues**
- ✅ Reduce unnecessary AI model calls
- ✅ Optimize prompt length and complexity
- ✅ Check for infinite loops in flow logic

## 🔗 Next Steps

- **[AI Node Configuration](ai-nodes.md)**: Deep dive into AI behaviors and model selection
- **Tool Integration**: Connect external services and APIs
- **API Documentation**: Integrate VortiDeck with other applications

---

Ready to build sophisticated AI workflows? Start experimenting with the Flow Builder! 🎨 