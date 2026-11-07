# VortiDeck documentation

Start with the [architecture guide](architecture/ARCHITECTURE_GUIDE.md). It describes the current ownership model, the Rust runtime decisions and the remaining gaps, and says which older proposals it replaces. Documents below that predate it are design history, not a description of the current implementation.

Internal planning and review notes (`development/`, `reviews/` and the root task ledger) are kept in the maintainers' repository and are not part of the public source.

## User guides

- [Getting started](user-guide/getting-started.md)
- [Flow builder](user-guide/flow-builder.md)
- [AI node configuration](user-guide/ai-nodes.md)

## Architecture and decisions

- [Architecture guide](architecture/ARCHITECTURE_GUIDE.md): current foundation and decision guide
- [Product and runtime decisions for 0.7](architecture/PRODUCT_RUNTIME_DECISIONS.md)
- [Hosting scope decision](architecture/HOSTING_SCOPE_DECISION.md)
- [CLI, MCP and shared application services](architecture/CLI_MCP_AND_SHARED_APPLICATION_SERVICES.md)
- [Flow architecture](architecture/FLOW_ARCHITECTURE.md)
- [Durable human approval](architecture/DURABLE_HUMAN_APPROVAL_ARCHITECTURE.md)
- [Marketplace, R2 and package lifecycle](architecture/MARKETPLACE_R2_RELEASE_AND_PLUGIN_LIFECYCLE.md)
- [Remote relay and gateway](architecture/REMOTE_RELAY_GATEWAY_ARCHITECTURE.md)
- [Overlay component system](architecture/OVERLAY_COMPONENT_SYSTEM_ARCHITECTURE.md)
- [Unified deck widget engine and plugin interaction](architecture/UNIFIED_DECK_WIDGET_ENGINE_AND_PLUGIN_INTERACTION_SPEC.md)
- [Installer and multiplatform release](WINDOWS_INSTALLER_AND_MULTIPLATFORM_RELEASE_ARCHITECTURE.md)

### AI and agents

- [AI copilot and MCP](architecture/AI_COPILOT_AND_MCP_ARCHITECTURE.md)
- [AI executor](architecture/AI_EXECUTOR_ARCHITECTURE.md)
- [Agent system vision](architecture/AGENT_SYSTEM_ARCHITECTURE.md)
- [Multi-model session management](architecture/multi-model-session-management.md)
- [Cloud AI suggestion registry](architecture/CLOUD_AI_SUGGESTION_REGISTRY_ARCHITECTURE.md)
- [Dynamic macro parameters and live flow debugging (research)](architecture/DYNAMIC_MACRO_AND_FLOW_DEBUGGING_RESEARCH.md)
- [Flow, stream action and integration direction](VORTIDECK_FLOW_STREAM_ACTION_INTEGRATION_DIRECTION.md)

## Plugins, nodes and widgets

- [Plugin architecture](plugins/plugin-architecture.md)
- [Plugin system explained](plugins/plugin-system-explained.md)
- [Node development guide](plugins/NODE_DEVELOPMENT_GUIDE.md)
- [Overlay widget authoring settings](plugins/OVERLAY_WIDGET_SETTINGS.md)
- [VoiceFlow telephony plugin](plugins/voiceflow-telephony-plugin.md)
- [Widget capability and security model](WIDGET_CAPABILITY_ARCHITECTURE.md)
- [Widget capability dialog plan](WIDGET_CAPABILITY_DIALOG_PLAN.md)
- [Widget and plugin creation assessment](WIDGET_CREATION_ASSESSMENT.md)
- Plugin SDK and examples: [packages/plugin-sdk](../packages/plugin-sdk/README.md) and [examples](../examples/README.md)

## Streaming, overlays and OBS

- [Overlay engine](features/streaming/OVERLAY_ENGINE_ARCHITECTURE.md)
- [Overlay data flow](features/streaming/OVERLAY_DATA_FLOW_CONCEPTS.md)
- [Overlay library UX redesign](OVERLAY_LIBRARY_UX_REDESIGN.md)
- [Initial state sync](features/streaming/INITIAL_STATE_SYNC_FLOW.md)
- [Custom platforms and webhooks](features/streaming/CUSTOM_PLATFORMS_AND_WEBHOOKS.md)
- [Backend session decision](features/streaming/BACKEND_SESSION_ARCHITECTURE_DECISION.md)
- [Streaming architecture analysis](features/streaming/STREAMING_ARCHITECTURE_ANALYSIS.md)
- [Streaming production readiness](features/streaming/STREAMING_PRODUCTION_READY_ARCHITECTURE.md)
- [Streaming improvement plan](features/streaming/STREAMNG_IMPROVEMENTS.md)
- [Latest streaming design notes](features/streaming/LATEST_DESIGN.md)
- [OBS sync and automation](OBS_SYNC_AND_AUTOMATION_SPECIFICATION.md)
- [OBS companion mDNS discovery](OBS_COMPANION_MDNS_DISCOVERY_ARCHITECTURE_AND_CLEANUP_PLAN.md)

## Contributing

See [CONTRIBUTING.md](../CONTRIBUTING.md) for setup, checks and pull request guidelines, and [SECURITY.md](../SECURITY.md) to report a vulnerability.
