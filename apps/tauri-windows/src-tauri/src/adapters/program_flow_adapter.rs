//! Program Flow Adapter
//!
//! Bridges between the database layer and VortiDeck Core flow system for programs

use std::collections::HashMap;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use vortideck_core::services::flow::{DatabaseProgram, DatabaseProgramProvider, FlowNodeProvider};
use vortideck_core::db::models::Program;
use vortideck_core::db::schema::programs;

/// Database connection type alias
type DbPool = Pool<ConnectionManager<SqliteConnection>>;

/// Adapter that loads programs from the database and creates a flow provider
pub struct ProgramFlowAdapter {
    db_pool: DbPool,
}

impl ProgramFlowAdapter {
    pub fn new(db_pool: DbPool) -> Self {
        Self { db_pool }
    }

    /// Load all programs from database and create a provider
    pub async fn create_provider(&self) -> Result<Box<dyn FlowNodeProvider>, String> {
        let programs = self.load_all_programs().await?;
        Ok(Box::new(DatabaseProgramProvider::from_programs(programs)))
    }

    /// Load all programs from the database
    pub async fn load_all_programs(&self) -> Result<Vec<DatabaseProgram>, String> {
        let db_programs = {
            let mut conn = self.db_pool.get()
                .map_err(|e| format!("Failed to get database connection: {}", e))?;

            programs::table
                .load::<Program>(&mut conn)
                .map_err(|e| format!("Failed to load programs from database: {}", e))?
        };

        // Convert database models to core models
        let core_programs = db_programs.into_iter()
            .map(|p| self.convert_db_program_to_core(p))
            .collect();

        Ok(core_programs)
    }

    /// Convert database Program model to core DatabaseProgram model
    fn convert_db_program_to_core(&self, db_program: Program) -> DatabaseProgram {
        DatabaseProgram {
            id: db_program.id,
            name: db_program.name,
            executable: db_program.executable,
            args: db_program.args,
            working_dir: db_program.working_dir,
            env_vars: db_program.env_vars,
            timeout_ms: db_program.timeout_ms,
            show_terminal: db_program.show_terminal,
            program_type: db_program.program_type,
            description: db_program.description,
            custom_icon: db_program.custom_icon,
        }
    }

    /// Get program count for logging
    pub async fn get_program_count(&self) -> Result<i64, String> {
        let mut conn = self.db_pool.get()
            .map_err(|e| format!("Failed to get database connection: {}", e))?;

        programs::table
            .count()
            .get_result(&mut conn)
            .map_err(|e| format!("Failed to count programs: {}", e))
    }
}

/// Convenience function to create a program provider from database
pub async fn create_program_provider(db_pool: DbPool) -> Result<Box<dyn FlowNodeProvider>, String> {
    let adapter = ProgramFlowAdapter::new(db_pool);
    adapter.create_provider().await
}

/// Function to refresh program registry (for dynamic updates)
pub async fn refresh_program_provider(
    flow_registry_state: &vortideck_core::services::flow::commands::FlowRegistryState,
    db_pool: DbPool
) -> Result<(), String> {
    println!("🔄 Refreshing program registry from database");

    // Clear existing program nodes
    {
        let mut registry = flow_registry_state.registry.lock().unwrap();
        registry.clear_provider_nodes("DatabaseProgramProvider");
    }

    // Create new provider with fresh data
    let program_provider = create_program_provider(db_pool).await?;

    // Register the new provider
    {
        let mut registry = flow_registry_state.registry.lock().unwrap();
        registry.register_provider(program_provider);
    }

    println!("✅ Program registry refreshed successfully");
    Ok(())
}