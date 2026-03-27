use crate::types::*;
use crate::interner::Interner;
use crate::events::Event;

pub fn extract_semantic_events(
    file_path: &str,
    _code: &str,
    interner: &mut Interner,
) -> Vec<Event> {
    let mut events = Vec::new();
    let file_id = interner.intern(file_path);
    
    // 1. CLEAR OLD STATE (Mandatory before applying deltas)
    events.push(Event::ClearFile { file_id });

    // Mock extraction
    let symbol_fqn = format!("{}::mock_func", file_path);
    let symbol_id = interner.intern(&symbol_fqn);

    events.push(Event::AddNode { id: symbol_id, kind: NodeType::Function, file_id });
    
    events
}
