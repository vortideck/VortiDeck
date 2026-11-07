/// Flow actions
#[derive(Debug, Clone)]
pub struct FlowAction {
    pub id: String,
    pub name: String,
    pub action_type: String,
}

impl FlowAction {
    pub fn new(id: String, name: String, action_type: String) -> Self {
        Self {
            id,
            name,
            action_type,
        }
    }
}
