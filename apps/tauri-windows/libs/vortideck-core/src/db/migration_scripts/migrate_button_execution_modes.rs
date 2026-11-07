use crate::db::utils::Database;
use crate::db::DbPool;
use diesel::prelude::*;
use serde_json::Value;

pub async fn migrate_button_execution_modes(pool: &DbPool) -> Result<(), Box<dyn std::error::Error>> {
    let db = Database::new_from_pool(pool.clone());
    
    // Get all buttons with MacroId actions
    let buttons = db.get_all_buttons_with_macro_actions().await?;
    
    println!("🔄 Starting migration of {} buttons with macro actions", buttons.len());
    
    for button in buttons {
        if let Ok(action) = button.get_action() {
            if let crate::db::models::Action::MacroId(macro_id) = action {
                // Get the macro to retrieve its execution mode
                if let Ok(Some(macro_record)) = db.get_macro_by_id(macro_id).await {
                    let execution_mode = match macro_record.macro_type.as_str() {
                        "RunOnce" => "run_once",
                        "RepeatWhilePressing" => "repeat_while_pressing", 
                        "Switch" => "switch",
                        "Queue" => "queue",
                        _ => "run_once",
                    };
                    
                    // Update the button's execution mode
                    let result = db.set_button_execution_mode(button.id, execution_mode, None).await;
                    match result {
                        Ok(_) => println!("✅ Migrated button {} execution mode to: {}", button.id, execution_mode),
                        Err(e) => println!("❌ Failed to migrate button {}: {}", button.id, e),
                    }
                }
            }
        }
    }
    
    println!("🎉 Button execution mode migration completed!");
    Ok(())
}