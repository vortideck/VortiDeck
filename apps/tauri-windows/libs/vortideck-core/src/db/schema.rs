// @generated automatically by Diesel CLI.

diesel::table! {
    action_input_contracts (reference_key) {
        reference_key -> Text,
        revision -> BigInt,
        contract -> Text,
        updated_at -> Text,
    }
}

diesel::table! {
    action_invocations (invocation_id) {
        invocation_id -> Text,
        parent_invocation_id -> Nullable<Text>,
        source -> Text,
        source_reference -> Nullable<Text>,
        workflow_run_id -> Nullable<Text>,
        node_id -> Nullable<Text>,
        resolved_action -> Text,
        integration_id -> Nullable<Text>,
        action_id -> Nullable<Text>,
        capability_version -> Nullable<Integer>,
        idempotency_key -> Nullable<Text>,
        resource_key -> Nullable<Text>,
        dispatch_status -> Text,
        receipt_semantics -> Nullable<Text>,
        request_id -> Nullable<Text>,
        provider_receipt_id -> Nullable<Text>,
        provider_outputs -> Nullable<Text>,
        normalized_status -> Nullable<Text>,
        error -> Nullable<Text>,
        recovery_decision -> Nullable<Text>,
        started_at -> Text,
        completed_at -> Nullable<Text>,
        execution_time_ms -> BigInt,
        created_at -> Text,
        updated_at -> Text,
    }
}

diesel::table! {
    overlay_media_queue (id) {
        id -> Text,
        invocation_id -> Text,
        queue_kind -> Text,
        command -> Text,
        payload -> Text,
        status -> Text,
        receipt_phase -> Nullable<Text>,
        provider_receipt_id -> Nullable<Text>,
        error -> Nullable<Text>,
        created_at -> Text,
        updated_at -> Text,
        started_at -> Nullable<Text>,
        completed_at -> Nullable<Text>,
    }
}

diesel::table! {
    actions (id) {
        id -> Integer,
        action_type -> Text,
        macro_id -> Nullable<Integer>,
        file_path -> Nullable<Text>,
        custom_data -> Nullable<Text>,
        execution_mode -> Nullable<Text>,
        execution_config -> Nullable<Text>,
    }
}

diesel::table! {
    active_deck (id) {
        id -> Integer,
        deck_id -> Integer,
    }
}

diesel::table! {
    app_config (key) {
        key -> Text,
        tls_enabled -> Bool,
        api_url -> Nullable<Text>,
        log_level -> Nullable<Text>,
        language -> Nullable<Text>,
        timezone -> Nullable<Text>,
        theme -> Nullable<Text>,
        show_instruction -> Bool,
        mcp_enabled -> Bool,
        plugin_hot_reload_enabled -> Bool,
        desktop_id -> Nullable<Text>,
        max_concurrent_actions -> Integer,
        max_action_timeout_ms -> Integer,
        relay_enabled -> Bool,
        streaming_enabled -> Bool,
        analytics_enabled -> Bool,
        analytics_consent_given -> Bool,
    }
}

diesel::table! {
    buttons (id) {
        id -> Integer,
        deck_id -> Integer,
        label -> Text,
        icon -> Nullable<Text>,
        background -> Nullable<Text>,
        position -> Integer,
        action -> Nullable<Text>,
        mode -> Nullable<Text>,
        execution_mode -> Nullable<Text>,
        execution_config -> Nullable<Text>,
        icon_updated_at -> Nullable<Integer>,
        icon_fit -> Text,
    }
}

diesel::table! {
    deck_settings (id) {
        id -> Integer,
        button_spacing -> Float,
        grid_margin -> Float,
        group_buttons -> Bool,
        maximize_button_space -> Bool,
        grid -> Text,
        deck_id -> Integer,
        buttons_padding -> Float,
        sides_enabled -> Bool,
        orientation_sync_enabled -> Bool,
        active_theme -> Nullable<Integer>,
    }
}

diesel::table! {
    decks (id) {
        id -> Integer,
        name -> Text,
    }
}

diesel::table! {
    deck_widget_documents (deck_id) {
        deck_id -> Integer,
        revision -> Integer,
        updated_at -> Text,
    }
}

diesel::table! {
    deck_items (id) {
        id -> Text,
        deck_id -> Integer,
        definition_id -> Text,
        package_id -> Nullable<Text>,
        contribution_id -> Nullable<Text>,
        config_json -> Text,
        config_version -> Integer,
        appearance_json -> Text,
        legacy_button_id -> Nullable<Integer>,
        enabled -> Bool,
    }
}

diesel::table! {
    deck_item_layouts (item_id, layout_key) {
        item_id -> Text,
        layout_key -> Text,
        x -> Integer,
        y -> Integer,
        col_span -> Integer,
        row_span -> Integer,
    }
}

diesel::table! {
    deck_item_bindings (id) {
        id -> Text,
        item_id -> Text,
        port_id -> Text,
        action_json -> Text,
        policy_json -> Text,
        transform_json -> Nullable<Text>,
    }
}

diesel::table! {
    deck_item_persistent_state (item_id) {
        item_id -> Text,
        state_json -> Text,
        updated_at -> Text,
    }
}

diesel::table! {
    flows (id) {
        id -> Text,
        name -> Text,
        description -> Nullable<Text>,
        data -> Text,
        created_at -> Text,
        updated_at -> Text,
        created_by -> Nullable<Text>,
        tags -> Nullable<Text>,
        version -> Text,
    }
}

diesel::table! {
    focus_windows (id) {
        id -> Integer,
        name -> Text,
        window_title -> Text,
        process_name -> Text,
        match_mode -> Text,
        created_at -> Text,
    }
}

diesel::table! {
    http_requests (id) {
        id -> Integer,
        name -> Text,
        method -> Text,
        url -> Text,
        headers -> Nullable<Text>,
        body -> Nullable<Text>,
        timeout_ms -> Nullable<Integer>,
        created_at -> Text,
        accepted_status_codes -> Nullable<Text>,
    }
}

diesel::table! {
    macros (id) {
        id -> Integer,
        macro_chain -> Text,
        name -> Text,
        macro_type -> Text,
    }
}

diesel::table! {
    saved_actions (id) {
        id -> Integer,
        stable_id -> Text,
        integration_id -> Text,
        name -> Text,
        action -> Text,
        params -> Text,
        capability_version -> Integer,
        connection_selector -> Nullable<Text>,
        presentation -> Text,
    }
}

diesel::table! {
    peers (id) {
        id -> Nullable<Integer>,
        client_id -> Text,
        ip -> Text,
        device_type -> Text,
        device -> Text,
        device_token -> Nullable<Text>,
        mobile_public_key -> Nullable<Text>,
        shared_secret -> Nullable<Text>,
    }
}

diesel::table! {
    remote_command_receipts (acceptance_id) {
        acceptance_id -> Text,
        peer_client_id -> Text,
        key_fingerprint -> Text,
        message_id -> Text,
        nonce -> Text,
        payload_digest -> Text,
        transport -> Text,
        status -> Text,
        sent_at -> BigInt,
        expires_at -> BigInt,
        receipt_json -> Text,
        accepted_at -> Text,
        updated_at -> Text,
    }
}

diesel::table! {
    programs (id) {
        id -> Integer,
        name -> Text,
        executable -> Text,
        args -> Nullable<Text>,
        working_dir -> Nullable<Text>,
        env_vars -> Nullable<Text>,
        timeout_ms -> Nullable<Integer>,
        show_terminal -> Bool,
        program_type -> Text,
        description -> Nullable<Text>,
        custom_icon -> Nullable<Text>,
    }
}

diesel::table! {
    chat_messages (id) {
        id -> Text,
        account_id -> Integer,
        session_id -> Nullable<Text>,
        session_run_index -> Nullable<Integer>,
        platform -> Text,
        user_id -> Text,
        user_name -> Text,
        user_display_name -> Text,
        message -> Text,
        timestamp -> Text,
        badges -> Nullable<Text>,
        is_moderator -> Integer,
        is_subscriber -> Integer,
        is_vip -> Integer,
        platform_color -> Nullable<Text>,
        emotes -> Nullable<Text>,
        mentions -> Nullable<Text>,
        created_at -> Text,
    }
}

diesel::table! {
    stream_sessions (id) {
        id -> Text,
        name -> Text,
        description -> Nullable<Text>,
        platforms -> Text,
        commands -> Text,
        settings -> Text,
        status -> Text,
        created_at -> Text,
        updated_at -> Text,
        current_run_started_at -> Nullable<Text>,
        paused_at -> Nullable<Text>,
        duration -> Nullable<Integer>,
        message_count -> Nullable<Integer>,
        command_executions -> Nullable<Integer>,
        runs -> Nullable<Text>,
        total_runs -> Nullable<Integer>,
        last_used_at -> Nullable<Text>,
        total_duration -> Nullable<Integer>,
        total_messages -> Nullable<Integer>,
        total_commands -> Nullable<Integer>,
    }
}

diesel::table! {
    themes (id) {
        id -> Integer,
        name -> Text,
        background_css -> Text,
        rotary_button_knob_css -> Text,
        rotary_button_pointer_css -> Text,
        click_button_button_css -> Text,
        click_button_icon_css -> Text,
        click_button_selected_css -> Text,
        rotary_button_ready_css -> Text,
    }
}

diesel::table! {
    webhook_events (id) {
        id -> Text,
        webhook_id -> Text,
        event_type -> Text,
        status -> Text,
        payload -> Nullable<Text>,
        response_code -> Nullable<Integer>,
        response_body -> Nullable<Text>,
        duration_ms -> Nullable<Integer>,
        error_message -> Nullable<Text>,
        created_at -> Text,
    }
}

diesel::table! {
    webhooks (id) {
        id -> Text,
        name -> Text,
        description -> Nullable<Text>,
        service -> Text,
        endpoint_url -> Text,
        inbound_secret -> Nullable<Text>,
        action -> Nullable<Text>,
        enabled -> Bool,
        created_at -> Text,
        updated_at -> Text,
        subscription_id -> Nullable<Text>,
        test_url -> Nullable<Text>,
        production_url -> Nullable<Text>,
        is_test_active -> Bool,
        test_expires_at -> Nullable<Text>,
        response_mode -> Text,
        allowed_methods -> Text,
    }
}

diesel::table! {
    package_permission_grants (package_id) {
        package_id -> Text,
        manifest_json -> Text,
        permissions_json -> Text,
        content_hash -> Text,
        trust -> Text,
        granted_at -> Text,
    }
}

diesel::table! {
    installed_package_releases (package_id) {
        package_id -> Text,
        installed_version -> Text,
        source -> Text,
        release_id -> Nullable<Text>,
        artifact_sha256 -> Text,
        marketplace_key_id -> Nullable<Text>,
        installed_at -> Text,
        updated_at -> Text,
        auto_update -> Bool,
        lifecycle_state -> Text,
        last_known_good_version -> Nullable<Text>,
        last_known_good_release_id -> Nullable<Text>,
        last_known_good_artifact_sha256 -> Nullable<Text>,
        last_known_good_marketplace_key_id -> Nullable<Text>,
        installed_content_sha256 -> Nullable<Text>,
    }
}

diesel::joinable!(webhook_events -> webhooks (webhook_id));
diesel::joinable!(deck_widget_documents -> decks (deck_id));
diesel::joinable!(deck_items -> decks (deck_id));
diesel::joinable!(deck_items -> buttons (legacy_button_id));
diesel::joinable!(deck_item_layouts -> deck_items (item_id));
diesel::joinable!(deck_item_bindings -> deck_items (item_id));
diesel::joinable!(deck_item_persistent_state -> deck_items (item_id));

diesel::allow_tables_to_appear_in_same_query!(
    action_input_contracts,
    action_invocations,
    actions,
    active_deck,
    app_config,
    buttons,
    chat_messages,
    deck_settings,
    deck_widget_documents,
    deck_items,
    deck_item_layouts,
    deck_item_bindings,
    deck_item_persistent_state,
    decks,
    flows,
    focus_windows,
    http_requests,
    macros,
    peers,
    programs,
    remote_command_receipts,
    saved_actions,
    stream_sessions,
    themes,
    webhook_events,
    webhooks,
    package_permission_grants,
    installed_package_releases,
    overlay_media_queue,
);
