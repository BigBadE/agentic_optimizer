use serde_json::Value;
use std::collections::HashSet;

/// A node in the output tree
#[derive(Clone, Debug)]
pub enum OutputNode {
    Step {
        id: String,
        step_type: StepType,
        content: String,
        children: Vec<OutputNode>,
    },
    #[allow(
        dead_code,
        reason = "ToolCall variant is part of public API for future use"
    )]
    ToolCall {
        id: String,
        tool_name: String,
        result: Option<ToolResult>,
    },
    Text {
        content: String,
    },
}

#[derive(Clone, Debug)]
pub struct ToolResult {
    pub success: bool,
    pub content: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StepType {
    Thinking,
    ToolCall,
    Output,
    Subtask,
}

impl StepType {
    pub fn from_str(text: &str) -> Self {
        match text {
            "Thinking" => Self::Thinking,
            "ToolCall" => Self::ToolCall,
            "Subtask" => Self::Subtask,
            _ => Self::Output,
        }
    }
}

/// Tree structure for task output
pub struct OutputTree {
    root: Vec<OutputNode>,
    selected_index: usize,
    collapsed_nodes: HashSet<String>,
    current_step_stack: Vec<String>, // Track nested steps
}

impl OutputTree {
    pub fn new() -> Self {
        Self {
            root: Vec::new(),
            selected_index: 0,
            collapsed_nodes: HashSet::new(),
            current_step_stack: Vec::new(),
        }
    }

    /// Add a new step to the tree
    pub fn add_step(&mut self, step_id: String, step_type: StepType, content: String) {
        let node = OutputNode::Step {
            id: step_id.clone(),
            step_type,
            content,
            children: Vec::new(),
        };

        if let Some(parent_id) = self.current_step_stack.last().cloned() {
            // Add as child to current parent
            if let Some(parent) = self.find_node_mut(&parent_id)
                && let OutputNode::Step { children, .. } = parent
            {
                children.push(node);
            }
        } else {
            // Add to root
            self.root.push(node);
        }

        // Push this step onto the stack for potential children
        self.current_step_stack.push(step_id);
    }

    /// Complete a step (pop from stack and auto-collapse if it's "analysis")
    pub fn complete_step(&mut self, step_id: &str) {
        if self.current_step_stack.last().map(String::as_str) == Some(step_id) {
            self.current_step_stack.pop();
        }

        // Auto-collapse "analysis" steps when they complete
        if step_id == "analysis" {
            self.collapsed_nodes.insert(step_id.to_string());
        }
    }

    /// Add a tool call
    #[allow(dead_code, reason = "Method is part of public API for future use")]
    pub fn add_tool_call(&mut self, tool_name: String, _args: Value) {
        let id = format!("tool_{}", self.root.len());
        let node = OutputNode::ToolCall {
            id,
            tool_name,
            result: None,
        };

        if let Some(parent_id) = self.current_step_stack.last().cloned() {
            if let Some(parent) = self.find_node_mut(&parent_id)
                && let OutputNode::Step { children, .. } = parent
            {
                children.push(node);
            }
        } else {
            self.root.push(node);
        }
    }

    /// Complete a tool call with result
    pub fn complete_tool_call(&mut self, tool_name: &str, result: &Value) {
        let success = result
            .get("success")
            .and_then(Value::as_bool)
            .unwrap_or(true);

        let content = result
            .get("content")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string();

        let tool_result = ToolResult { success, content };

        // Find the most recent tool call with this name
        if let Some(node) = self.find_tool_call_mut(tool_name)
            && let OutputNode::ToolCall {
                result: result_slot,
                ..
            } = node
        {
            *result_slot = Some(tool_result);
        }
    }

    /// Add plain text output
    pub fn add_text(&mut self, content: String) {
        let node = OutputNode::Text { content };

        if let Some(parent_id) = self.current_step_stack.last().cloned() {
            if let Some(parent) = self.find_node_mut(&parent_id)
                && let OutputNode::Step { children, .. } = parent
            {
                children.push(node);
            }
        } else {
            self.root.push(node);
        }
    }

    /// Get flattened list of visible nodes with their depth
    pub fn flatten_visible_nodes(&self) -> Vec<(OutputNodeRef<'_>, usize)> {
        let mut result = Vec::new();

        for node in &self.root {
            self.flatten_node(node, 0, &mut result, &[]);
        }

        result
    }

    fn flatten_node<'node>(
        &'node self,
        node: &'node OutputNode,
        depth: usize,
        result: &mut Vec<(OutputNodeRef<'node>, usize)>,
        parent_states: &[bool],
    ) {
        let node_ref = OutputNodeRef {
            node,
            is_last: false,
            parent_states: parent_states.to_vec(),
        };
        result.push((node_ref, depth));

        if !self.is_collapsed(node)
            && let Some(children) = Self::get_children(node)
        {
            let child_count = children.len();

            for (idx, child) in children.iter().enumerate() {
                let is_last = idx == child_count - 1;
                let mut new_parent_states = parent_states.to_vec();
                new_parent_states.push(is_last);
                self.flatten_node(child, depth + 1, result, &new_parent_states);
            }
        }
    }

    pub fn is_collapsed(&self, node: &OutputNode) -> bool {
        Self::get_node_id(node).is_some_and(|id| self.collapsed_nodes.contains(id))
    }

    fn get_children(node: &OutputNode) -> Option<&Vec<OutputNode>> {
        match node {
            OutputNode::Step { children, .. } => Some(children),
            _ => None,
        }
    }

    fn get_node_id(node: &OutputNode) -> Option<&str> {
        match node {
            OutputNode::Step { id, .. } | OutputNode::ToolCall { id, .. } => Some(id.as_str()),
            OutputNode::Text { .. } => None,
        }
    }

    fn find_node_mut(&mut self, target_id: &str) -> Option<&mut OutputNode> {
        for node in &mut self.root {
            if let Some(found) = Self::find_node_recursive(node, target_id) {
                return Some(found);
            }
        }
        None
    }

    fn find_node_recursive<'node>(
        node: &'node mut OutputNode,
        target_id: &str,
    ) -> Option<&'node mut OutputNode> {
        if let Some(id) = match node {
            OutputNode::Step { id, .. } | OutputNode::ToolCall { id, .. } => Some(id.as_str()),
            OutputNode::Text { .. } => None,
        } && id == target_id
        {
            return Some(node);
        }

        if let OutputNode::Step { children, .. } = node {
            for child in children {
                if let Some(found) = Self::find_node_recursive(child, target_id) {
                    return Some(found);
                }
            }
        }

        None
    }

    fn find_tool_call_mut(&mut self, tool_name: &str) -> Option<&mut OutputNode> {
        for node in self.root.iter_mut().rev() {
            if let Some(found) = Self::find_tool_call_recursive(node, tool_name) {
                return Some(found);
            }
        }
        None
    }

    fn find_tool_call_recursive<'node>(
        node: &'node mut OutputNode,
        tool_name: &str,
    ) -> Option<&'node mut OutputNode> {
        if let OutputNode::ToolCall {
            tool_name: name,
            result,
            ..
        } = node
            && name == tool_name
            && result.is_none()
        {
            return Some(node);
        }

        if let OutputNode::Step { children, .. } = node {
            for child in children.iter_mut().rev() {
                if let Some(found) = Self::find_tool_call_recursive(child, tool_name) {
                    return Some(found);
                }
            }
        }

        None
    }

    /// Navigation methods
    pub fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn move_down(&mut self) {
        let visible_count = self.flatten_visible_nodes().len();
        if self.selected_index + 1 < visible_count {
            self.selected_index += 1;
        }
    }

    pub fn move_to_start(&mut self) {
        self.selected_index = 0;
    }

    pub fn move_to_end(&mut self) {
        let visible_count = self.flatten_visible_nodes().len();
        self.selected_index = visible_count.saturating_sub(1);
    }

    pub fn page_up(&mut self, page_size: usize) {
        self.selected_index = self.selected_index.saturating_sub(page_size);
    }

    pub fn page_down(&mut self, page_size: usize) {
        let visible_count = self.flatten_visible_nodes().len();
        self.selected_index =
            (self.selected_index + page_size).min(visible_count.saturating_sub(1));
    }

    pub fn expand_selected(&mut self) {
        let visible = self.flatten_visible_nodes();
        if let Some((node_ref, _)) = visible.get(self.selected_index)
            && let Some(id) = Self::get_node_id(node_ref.node)
        {
            let id_owned = id.to_string();
            self.collapsed_nodes.remove(&id_owned);
        }
    }

    pub fn collapse_selected(&mut self) {
        let visible = self.flatten_visible_nodes();
        if let Some((node_ref, _)) = visible.get(self.selected_index)
            && let Some(id) = Self::get_node_id(node_ref.node)
        {
            let id_owned = id.to_string();
            self.collapsed_nodes.insert(id_owned);
        }
    }

    pub fn toggle_selected(&mut self) {
        let visible = self.flatten_visible_nodes();
        if let Some((node_ref, _)) = visible.get(self.selected_index)
            && let Some(id) = Self::get_node_id(node_ref.node)
        {
            let id_owned = id.to_string();
            if self.collapsed_nodes.contains(&id_owned) {
                self.collapsed_nodes.remove(&id_owned);
            } else {
                self.collapsed_nodes.insert(id_owned);
            }
        }
    }

    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    /// Get all text content as a flat string (for saving)
    pub fn to_text(&self) -> String {
        let mut lines = Vec::new();
        for node in &self.root {
            Self::node_to_text(node, 0, &mut lines);
        }

        lines.join("\n")
    }

    fn node_to_text(node: &OutputNode, depth: usize, lines: &mut Vec<String>) {
        let indent = "  ".repeat(depth);
        let icon = node.get_icon(false);
        let content = node.get_content();
        lines.push(format!("{indent}{icon} {content}"));

        if let Some(children) = Self::get_children(node) {
            for child in children {
                Self::node_to_text(child, depth + 1, lines);
            }
        }
    }
}

impl Default for OutputTree {
    fn default() -> Self {
        Self::new()
    }
}

/// Reference to a node with rendering context
pub struct OutputNodeRef<'node> {
    pub node: &'node OutputNode,
    pub is_last: bool,
    pub parent_states: Vec<bool>,
}

impl OutputNode {
    pub fn get_icon(&self, is_collapsed: bool) -> &'static str {
        match self {
            Self::Step {
                step_type,
                children,
                ..
            } => {
                if children.is_empty() {
                    match step_type {
                        StepType::Thinking => "[*]",
                        StepType::ToolCall => "[T]",
                        StepType::Output => "[>]",
                        StepType::Subtask => "[S]",
                    }
                } else if is_collapsed {
                    "[+]"
                } else {
                    "[-]"
                }
            }
            Self::ToolCall { result, .. } => match result {
                Some(result_val) if result_val.success => "[+]",
                Some(_) => "[X]",
                None => "[T]",
            },
            Self::Text { .. } => "  ",
        }
    }

    pub fn get_content(&self) -> String {
        match self {
            Self::ToolCall {
                tool_name, result, ..
            } => result.as_ref().map_or_else(
                || format!("Calling tool: {tool_name}"),
                |res| format!("{}: {}", tool_name, res.content),
            ),
            Self::Step { content, .. } | Self::Text { content, .. } => content.clone(),
        }
    }
}

pub fn build_tree_prefix(depth: usize, is_last: bool, parent_states: &[bool]) -> String {
    let mut prefix = String::new();

    for index in 0..depth {
        if index < parent_states.len() && !parent_states[index] {
            prefix.push_str("│ ");
        } else {
            prefix.push_str("  ");
        }
    }

    if depth > 0 {
        if is_last {
            prefix.push_str("└─ ");
        } else {
            prefix.push_str("├─ ");
        }
    }

    prefix
}
