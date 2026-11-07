use serde::Serialize;

#[derive(Serialize)]
pub struct ToolNodeInfo {
    pub id: String,   // The actual backend node ID
    pub name: String, // Display name
    pub description: String,
    pub parameters: serde_json::Value,
    pub type_name: String,
    pub color: String,
    pub type_color: String,
    pub outputs: Vec<crate::services::flow::traits::NodeOutput>,
    /// Human-readable label for the stable category identifier.
    pub category_display_name: Option<String>,
    /// Optional second-level grouping within the category.
    pub subcategory: Option<String>,
}
