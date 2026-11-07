// src/commands/mod.rs
pub mod api_commands;
pub mod assistant_commands;
pub mod auth_commands;
pub mod context_menu_commands;
pub mod conversation_commands;
pub mod discover_websocket;
pub mod error_commands;
pub mod focus_window_commands;
pub mod generate_qr_code;
pub mod get_installed_apps;
pub mod http_request_commands;
pub mod obs_banner_commands;
pub mod obs_companion_commands;
pub mod open_in_new_window;
pub mod open_logs;
pub mod open_mobile_simulator;
pub mod open_overlay_window;
pub mod overlay_commands;
pub mod overlay_authoring;
pub mod plugin_commands;
pub mod process_commands;
pub mod relay_commands;
pub mod retrieve_peers;
pub mod state_commands;
pub mod streaming_commands;
pub mod universal_action_commands;
pub mod universal_integration_commands;
pub mod universal_system_commands;
pub mod webhook_commands;
pub mod widget_commands;
pub mod window_commands;
pub mod workspace_commands;

// API commands
pub use api_commands::{http_request, send_webhook, send_webhook_event};

// Auth commands
pub use auth_commands::{
    exchange_auth_code, get_access_token, get_refresh_token, logout_user, refresh_access_token,
    set_refresh_token,
};

pub use discover_websocket::discover_websocket;
pub use generate_qr_code::generate_qr_code;
pub use get_installed_apps::get_installed_apps;
pub use open_in_new_window::open_in_new_window;
pub use open_logs::open_logs;
pub use open_mobile_simulator::open_mobile_simulator;
pub use open_overlay_window::open_overlay_window;
pub use retrieve_peers::{retrieve_active_peers, retrieve_peers};

// Universal system commands
pub use universal_system_commands::{
    execute_system_action, get_available_actions, get_system_info_universal_cmd,
    register_universal_system_commands, set_volume,
};

// New state management commands
pub use state_commands::{
    broadcast_message_new,
    check_plugin_permission,
    compare_state_systems,
    enable_new_state_system,
    get_all_feature_usage_stats,
    get_feature_usage_stats,
    get_mobile_simulator_pin_status,
    get_peers_new,
    get_premium_status_new,
    get_public_state_for_plugin,
    get_server_info_new,
    get_state_system_status,
    initialize_default_features,
    pin_mobile_simulator,
    reset_feature_usage,
    set_obs_status_new,
    set_plugin_permissions,
    unpin_mobile_simulator,
    use_macro_recording_feature,
    use_ui_button_feature,
    validate_advanced_integrations_feature,
    validate_custom_feature,
    validate_macro_recording_feature,
    // Universal Feature Architecture commands
    validate_ui_buttons_feature,
};

// Universal Integration commands
pub use universal_integration_commands::{
    get_action_runtime_readiness, get_all_integration_statuses, get_connected_integrations,
    get_integration_capabilities, get_integration_status, get_integration_summary,
    is_integration_connected, refresh_integration_statuses, resolve_integration_capability_options,
};

// Universal Action commands (database operations)
pub use universal_action_commands::{
    disable_flows_using_saved_action, get_action_invocation, get_all_saved_integration_actions,
    get_integration_action_by_id, get_saved_action_dependencies,
    get_saved_integration_action_by_stable_id, get_saved_integration_actions,
    invoke_action_reference, list_action_invocations, rebind_saved_integration_action,
    remove_integration_action_from_db, replace_saved_action_references,
    save_integration_action_to_db, update_integration_action_in_db,
};

// Overlay commands (Clean Architecture)
pub use overlay_commands::{
    // add_overlay_element removed - V3 uses streaming elements
    add_overlay_element_v3, // V3 scalable element with HTML
    clear_overlay_elements,
    generate_overlay_html_command,
    // add_test_overlay_element removed - V3 uses streaming elements
    // trigger_test_overlay_ad,

    // V3 Streaming Elements System
    get_overlay_status,
    remove_overlay_element,
    send_overlay_to_obs,
    set_overlay_design,
    start_overlay_server_command,
    update_overlay_element,
};

// Process commands (Process Management and Execution)
pub use process_commands::{
    execute_program, get_all_processes, get_process, get_process_count, get_processes_by_button,
    get_processes_by_deck, get_running_processes, kill_all_processes, kill_process,
    kill_processes_by_button, kill_processes_by_deck,
};

// AI Assistant commands
pub use assistant_commands::{
    assistant_fill_value, get_assistant_context, get_assistant_suggestions,
    get_default_chat_settings, open_assistant_window, read_mcp_config, save_default_chat_settings,
    send_chat_message, update_chat_settings,
};

// Conversation Management commands
pub use conversation_commands::{
    attach_conversation_to_session, create_and_attach_conversation, create_conversation,
    delete_conversation, list_conversations, load_conversation_messages, update_conversation_title,
};

// Window Management commands
pub use window_commands::{
    close_streaming_window, get_monitors_info, move_streaming_window_to_monitor,
    open_streaming_window, toggle_streaming_window_always_on_top,
};

// HTTP Request commands
pub use http_request_commands::{
    delete_http_request, execute_http_request_by_id, get_http_request_by_id, list_http_requests,
    save_http_request, test_http_request,
};

// Webhook commands
pub use webhook_commands::{
    check_external_webhook, create_webhook, deactivate_test_mode, delete_webhook, get_webhook,
    get_webhook_statistics, get_webhooks, register_webhook_with_service,
    set_webhook_allowed_methods, set_webhook_response_mode, unregister_webhook_from_service,
    update_webhook,
};

// Focus Window commands
pub use focus_window_commands::{
    create_focus_window, delete_focus_window, get_focus_window_by_id, get_focus_windows,
    get_visible_windows, update_focus_window,
};

// Streaming commands
pub use streaming_commands::{
    streaming_activate_session,
    streaming_ban_user,
    streaming_check_auto_resume,
    streaming_check_conflicts,
    // WebSocket
    streaming_connect,
    streaming_create_session,
    streaming_delete_message,
    streaming_delete_session,
    streaming_disconnect,
    streaming_end_session,
    streaming_find_session_by_account,
    streaming_get_active_sessions,
    // Session Manager (auto-persists to database)
    streaming_get_all_sessions,
    streaming_get_draft_sessions,
    // Message Store
    streaming_get_messages,
    streaming_get_messages_for_accounts,
    streaming_get_recent_sessions,
    streaming_get_session,
    streaming_get_session_account_ids,
    streaming_is_session_live,
    streaming_pause_session,
    streaming_resume_session,
    // WebSocket Send Operations
    streaming_send_chat_message,
    streaming_start_session,
    streaming_timeout_user,
    streaming_update_session,
    streaming_update_token,
    update_data_subscriptions,
};

// Workspace commands
pub use workspace_commands::{
    get_workspace_input_dir, get_workspace_output_dir, get_workspace_root, get_workspace_temp_dir,
    open_workspace_directory, WorkspaceState,
};

// Context Menu commands (Windows 11 modern context menu management)
pub use context_menu_commands::{
    add_context_menu_item, get_context_menu_config, get_context_menu_items,
    get_context_menu_status, register_context_menu_extension, remove_context_menu_item,
    reorder_context_menu_items, toggle_context_menu_item, unregister_context_menu_extension,
    update_context_menu_item,
};

// Package capability commands (grant/revoke/query durable consent)
pub use widget_commands::{
    get_widget_permissions, grant_package_permissions, list_available_widgets,
    list_package_discovery_errors, list_packages, revoke_widget,
};

// Error reporting (frontend crash → native dialog)
pub use error_commands::report_frontend_crash;
