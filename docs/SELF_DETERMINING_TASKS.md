# Self-Determining Task System - Implementation Design

## Executive Summary

Instead of decomposing tasks upfront, let **tasks determine their own execution path** during runtime. The first execution can decide:
- Whether to spawn subtasks
- How complex the task really is
- Whether to elevate to a higher tier
- Whether to complete immediately

## Current State Analysis

### What Exists Now

**Location**: `crates/merlin-routing/src/analyzer/decompose.rs`

**Current Flow**:
```
User Request → IntentExtractor → TaskDecomposer → [Task, Task, Task]
                                      ↓
                              Upfront Decomposition
```

**Problems**:
1. **Over-decomposition**: "say hi" → 3 tasks
2. **Pre-determined complexity**: Can't adapt based on actual findings
3. **No runtime flexibility**: Path fixed before execution
4. **Rigid patterns**: Refactor = always 3 tasks, Fix = always 3 tasks

**Current Decomposer Logic**:
```rust
match &intent.action {
    Action::Refactor => vec![Analyze, Refactor, Test],  // Always 3
    Action::Create => vec![Design, Implement, Test],     // Always 3
    Action::Fix => vec![Diagnose, Fix, Verify],         // Always 3
    _ => vec![SingleTask],                              // Always 1
}
```

## Industry Patterns

### 1. AutoGPT - Dynamic Task Generation

**Approach**: Tasks create subtasks during execution
```python
class Task:
    async def execute(self):
        result = await self.attempt()
        
        # Analyze result and decide next steps
        if result.needs_breakdown:
            subtasks = self.generate_subtasks(result)
            for subtask in subtasks:
                await subtask.execute()
        
        return result
```

**Key Features**:
- Tasks spawn children based on findings
- Recursive decomposition
- Depth limits prevent infinite loops

### 2. LangChain Agents - ReAct Pattern

**Approach**: Reason → Act → Observe loop
```python
while not done:
    thought = agent.think(observation)  # "I need to check X first"
    
    if thought.suggests_subtask:
        action = agent.decide_action(thought)
        observation = execute(action)
    else:
        done = True
```

**Key Features**:
- Self-reflection before acting
- Can decide to gather more info
- Adaptive based on observations

### 3. CrewAI - Delegating Tasks

**Approach**: Agents can delegate to specialized agents
```python
class Agent:
    def execute(self, task):
        # Try to solve
        if self.can_handle(task):
            return self.solve(task)
        
        # Delegate to specialist
        specialist = self.find_specialist(task)
        return specialist.execute(task)
```

**Key Features**:
- Self-assessment of capability
- Dynamic delegation
- Specialized routing

### 4. Microsoft AutoGen - Conversation-Driven

**Approach**: Multi-agent conversations
```python
user_proxy.initiate_chat(
    assistant,
    message="Solve this problem"
)

# Assistant can:
# - Ask for clarification (spawn info-gathering task)
# - Delegate to code executor (spawn execution task)
# - Break down complex problems (spawn subtasks)
```

**Key Features**:
- Conversational task breakdown
- Dynamic agent assignment
- Adaptive complexity

## Proposed Architecture: Self-Determining Tasks

### Core Concept

**Tasks are autonomous agents** that:
1. **Assess** themselves during execution
2. **Decide** if they need help (spawn subtasks)
3. **Elevate** if they're too complex for current tier
4. **Complete** when they've solved the problem

### Execution Flow

```
User Request → Initial Task (Tier 1)
                    ↓
              Execute & Assess
                    ↓
        ┌───────────┴───────────┐
        ↓                       ↓
    Can Handle             Too Complex
        ↓                       ↓
    Complete          ┌─────────┴─────────┐
                      ↓                   ↓
                Spawn Subtasks      Elevate Tier
                      ↓                   ↓
              [Task, Task, ...]    Re-execute at Tier 2
```

### Task Lifecycle States

```rust
pub enum TaskState {
    Created,              // Just initialized
    Assessing,            // Analyzing what to do
    Executing,            // Doing the work
    SpawningSubtasks,     // Breaking down
    AwaitingSubtasks,     // Waiting for children
    Elevating,            // Moving to higher tier
    Completed,            // Done
    Failed,               // Error
}
```

### Decision Framework

```rust
pub struct TaskDecision {
    pub action: TaskAction,
    pub reasoning: String,
    pub confidence: f32,
}

pub enum TaskAction {
    // Complete immediately
    Complete { result: String },
    
    // Spawn subtasks
    Decompose {
        subtasks: Vec<SubtaskSpec>,
        execution_mode: ExecutionMode,  // Sequential or Parallel
    },
    
    // Elevate to higher tier
    Elevate {
        reason: ElevationReason,
        suggested_tier: ModelTier,
    },
    
    // Gather more information
    GatherContext {
        context_needs: Vec<ContextRequest>,
    },
    
    // Delegate to specialist
    Delegate {
        specialist_type: SpecialistType,
        delegation_context: String,
    },
}

pub enum ElevationReason {
    TooComplex,           // Beyond current tier capability
    RequiresReasoning,    // Need stronger model for logic
    RequiresCreativity,   // Need more creative model
    RequiresKnowledge,    // Need broader knowledge base
}
```

## Implementation Plan

### Phase 1: Self-Assessment Capability

**Add to Task**:
```rust
pub struct Task {
    pub id: TaskId,
    pub description: String,
    pub complexity: Complexity,
    pub state: TaskState,
    pub tier: ModelTier,
    
    // NEW: Self-determination fields
    pub decision_history: Vec<TaskDecision>,
    pub spawned_children: Vec<TaskId>,
    pub elevation_reason: Option<ElevationReason>,
}
```

**Create Assessment Engine**:
```rust
// crates/merlin-routing/src/executor/self_assess.rs

pub struct SelfAssessor {
    tier: ModelTier,
}

impl SelfAssessor {
    pub async fn assess_task(&self, task: &Task, context: &ExecutionContext) -> TaskDecision {
        // Prompt the model to decide what to do
        let prompt = self.build_assessment_prompt(task, context);
        let response = self.tier.execute(&prompt).await?;
        
        // Parse model's decision
        self.parse_decision(response)
    }
    
    fn build_assessment_prompt(&self, task: &Task, context: &ExecutionContext) -> String {
        format!(r#"
You are executing this task: "{}"

Current context:
- Available tools: {:?}
- Current tier: {}
- Execution history: {}

Assess this task and decide ONE of:
1. COMPLETE: If you can solve it immediately
2. DECOMPOSE: If it needs breaking into subtasks (specify subtasks)
3. ELEVATE: If it's too complex for tier {} (specify reason)
4. GATHER: If you need more information (specify what)

Respond in JSON:
{{
    "action": "COMPLETE" | "DECOMPOSE" | "ELEVATE" | "GATHER",
    "reasoning": "why you chose this action",
    "details": {{ ... action-specific details ... }}
}}
"#,
            task.description,
            context.available_tools,
            self.tier,
            context.execution_history,
            self.tier
        )
    }
}
```

### Phase 2: Dynamic Task Spawning

**Modified AgentExecutor**:
```rust
// crates/merlin-routing/src/agent/executor.rs

impl AgentExecutor {
    pub async fn execute_self_determining(
        &self,
        task: Task,
        ui_channel: UiChannel,
    ) -> Result<TaskResult> {
        let mut current_task = task;
        
        loop {
            // Step 1: Assess what to do
            ui_channel.send(UiEvent::TaskStepStarted {
                task_id: current_task.id,
                step_id: "assess".to_string(),
                step_type: "Assessing".to_string(),
                content: "Analyzing task complexity and requirements...".to_string(),
            });
            
            let decision = self.assessor.assess_task(&current_task, &self.context).await?;
            
            // Step 2: Execute decision
            match decision.action {
                TaskAction::Complete { result } => {
                    return Ok(TaskResult {
                        task_id: current_task.id,
                        response: Response { text: result, .. },
                        ..
                    });
                }
                
                TaskAction::Decompose { subtasks, execution_mode } => {
                    return self.execute_with_subtasks(
                        current_task,
                        subtasks,
                        execution_mode,
                        ui_channel
                    ).await;
                }
                
                TaskAction::Elevate { reason, suggested_tier } => {
                    // Re-execute at higher tier
                    current_task.tier = suggested_tier;
                    current_task.elevation_reason = Some(reason);
                    
                    ui_channel.send(UiEvent::TaskElevated {
                        task_id: current_task.id,
                        from_tier: self.tier,
                        to_tier: suggested_tier,
                        reason: reason.to_string(),
                    });
                    
                    // Get executor for higher tier
                    let elevated_executor = self.router.get_executor(suggested_tier)?;
                    return elevated_executor.execute_self_determining(
                        current_task,
                        ui_channel
                    ).await;
                }
                
                TaskAction::GatherContext { context_needs } => {
                    // Gather required context, then re-assess
                    self.gather_context(context_needs).await?;
                    continue;  // Loop back to re-assess with new context
                }
            }
        }
    }
    
    async fn execute_with_subtasks(
        &self,
        parent_task: Task,
        subtask_specs: Vec<SubtaskSpec>,
        execution_mode: ExecutionMode,
        ui_channel: UiChannel,
    ) -> Result<TaskResult> {
        // Spawn subtasks
        let subtasks: Vec<Task> = subtask_specs
            .into_iter()
            .map(|spec| Task {
                id: TaskId::new(),
                description: spec.description,
                complexity: spec.complexity,
                state: TaskState::Created,
                tier: spec.tier.unwrap_or(parent_task.tier),
                ..Default::default()
            })
            .collect();
        
        // Notify UI
        for subtask in &subtasks {
            ui_channel.task_started_with_parent(
                subtask.id,
                subtask.description.clone(),
                Some(parent_task.id)
            );
        }
        
        // Execute based on mode
        let results = match execution_mode {
            ExecutionMode::Sequential => {
                self.execute_sequential(subtasks, ui_channel.clone()).await?
            }
            ExecutionMode::Parallel => {
                self.execute_parallel(subtasks, ui_channel.clone()).await?
            }
        };
        
        // Synthesize results
        self.synthesize_subtask_results(parent_task, results).await
    }
}
```

### Phase 3: Smart Elevation

**Elevation Decision Logic**:
```rust
pub struct ElevationStrategy {
    complexity_threshold: f32,
    tier_capabilities: HashMap<ModelTier, Capabilities>,
}

impl ElevationStrategy {
    pub fn should_elevate(&self, task: &Task, current_tier: ModelTier) -> Option<ModelTier> {
        let required_capabilities = self.analyze_requirements(task);
        let current_capabilities = &self.tier_capabilities[&current_tier];
        
        // Check if current tier can handle it
        if current_capabilities.can_handle(&required_capabilities) {
            return None;  // No elevation needed
        }
        
        // Find appropriate tier
        for tier in ModelTier::ascending_from(current_tier) {
            if self.tier_capabilities[&tier].can_handle(&required_capabilities) {
                return Some(tier);
            }
        }
        
        None
    }
    
    fn analyze_requirements(&self, task: &Task) -> RequiredCapabilities {
        RequiredCapabilities {
            reasoning_depth: self.estimate_reasoning_depth(task),
            context_window: self.estimate_context_needs(task),
            domain_knowledge: self.identify_domain(task),
            creativity_level: self.estimate_creativity_needs(task),
        }
    }
}
```

### Phase 4: UI Integration

**New UI Events**:
```rust
pub enum UiEvent {
    // ... existing events ...
    
    // NEW: Self-determination events
    TaskAssessing {
        task_id: TaskId,
        assessment_stage: String,
    },
    
    TaskDecided {
        task_id: TaskId,
        decision: TaskAction,
        reasoning: String,
    },
    
    TaskElevated {
        task_id: TaskId,
        from_tier: ModelTier,
        to_tier: ModelTier,
        reason: String,
    },
    
    SubtasksSpawned {
        parent_id: TaskId,
        subtask_ids: Vec<TaskId>,
        execution_mode: ExecutionMode,
    },
}
```

**Output Tree Display**:
```
[>] Task: Fix complex bug
  [*] Assessing: Analyzing task complexity...
  [!] Decision: DECOMPOSE (Reason: Bug requires investigation first)
  
  ├─ [>] Subtask: Investigate bug reproduction
  │   [*] Assessing: Can handle with current tools
  │   [!] Decision: COMPLETE
  │   [+] Result: Found bug in parser.rs line 42
  │
  ├─ [>] Subtask: Analyze root cause
  │   [*] Assessing: Requires deeper reasoning
  │   [↑] Elevated: Tier1 → Tier2 (Reason: Complex logic analysis)
  │   [*] Analyzing with GPT-4...
  │   [+] Result: Race condition in token handler
  │
  └─ [>] Subtask: Implement fix
      [*] Assessing: Straightforward implementation
      [!] Decision: COMPLETE
      [T] Tool: write_file
      [+] Result: Fixed in parser.rs
```

## Comparison: Before vs. After

### Before (Static Decomposition)

**"say hi"**:
```
Request → Decomposer → [
    "Analyze greeting request",
    "Generate greeting response",  
    "Format output"
]
→ 3 tasks executed
```

**Problems**:
- Unnecessary decomposition
- Fixed pattern regardless of simplicity
- No adaptation

### After (Self-Determining)

**"say hi"**:
```
Request → Initial Task → Assess → Decision: COMPLETE
→ 1 task, immediate response
```

**"fix complex authentication bug"**:
```
Request → Initial Task → Assess → Decision: DECOMPOSE
    ├─ Subtask: Reproduce bug → Assess → COMPLETE
    ├─ Subtask: Analyze root cause → Assess → ELEVATE (Tier1→Tier2)
    │   → Higher tier analysis → Decision: COMPLETE
    └─ Subtask: Implement fix → Assess → DECOMPOSE
        ├─ Update auth logic → COMPLETE
        ├─ Add tests → COMPLETE
        └─ Verify security → ELEVATE (Tier2→Tier3) → COMPLETE
```

**Benefits**:
- Adaptive to actual complexity
- Minimal overhead for simple tasks
- Intelligent elevation when needed

## Implementation Roadmap

### Week 1: Foundation
- [ ] Add `TaskState` enum
- [ ] Add `TaskDecision` types
- [ ] Create `SelfAssessor` struct
- [ ] Implement assessment prompt generation

### Week 2: Core Logic
- [ ] Implement `execute_self_determining()`
- [ ] Add subtask spawning logic
- [ ] Implement elevation strategy
- [ ] Add decision parsing

### Week 3: Integration
- [ ] Update UI events for new flow
- [ ] Add output tree visualization
- [ ] Integrate with existing orchestrator
- [ ] Add tests for decision paths

### Week 4: Refinement
- [ ] Tune assessment prompts
- [ ] Optimize elevation thresholds
- [ ] Add metrics/logging
- [ ] Performance testing

## Migration Strategy

### Phase 1: Hybrid Mode
Keep both systems, use feature flag:
```rust
if config.use_self_determining {
    executor.execute_self_determining(task).await
} else {
    executor.execute_traditional(task).await
}
```

### Phase 2: Gradual Rollout
1. Enable for simple requests only
2. Monitor and tune
3. Enable for medium complexity
4. Full migration

### Phase 3: Deprecate Old System
Once stable, remove static decomposer

## Success Metrics

**Effectiveness**:
- Reduced task count for simple requests (target: 1 task for "say hi")
- Appropriate decomposition for complex tasks
- Correct tier elevation (measure accuracy)

**Performance**:
- Latency: Self-assessment should add <500ms
- Token usage: Efficient elevation decisions
- Success rate: >95% correct decisions

**UX**:
- User satisfaction with task transparency
- Clearer understanding of agent reasoning
- Faster results for simple requests

## Security Considerations

**Prompt Injection**:
- Validate JSON responses strictly
- Sanitize task descriptions
- Limit recursive depth

**Resource Limits**:
- Max subtasks per task: 10
- Max recursion depth: 5
- Timeout per assessment: 10s

**Cost Control**:
- Track elevation frequency
- Alert on excessive elevations
- Budget limits per task tree

## Conclusion

Self-determining tasks transform Merlin from a **static decomposer** to an **adaptive problem solver**:

✅ **Simple tasks stay simple** (1 task for "say hi")  
✅ **Complex tasks get proper breakdown** (assessed during execution)  
✅ **Smart tier elevation** (only when truly needed)  
✅ **Transparent decision-making** (users see reasoning)  
✅ **Adaptive to reality** (not pre-determined patterns)

This mirrors how **human developers work**: assess the problem, decide if help is needed, delegate when appropriate, and escalate when stuck.
