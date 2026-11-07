/// Flow state management
#[derive(Debug, Clone)]
pub struct FlowState {
    pub id: String,
    pub name: String,
    pub active: bool,
}

impl FlowState {
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            active: false,
        }
    }
}
