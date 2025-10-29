// TypeScript type definitions for agent responses

// Self-determination response types
// Agent returns one of these to determine task execution strategy

interface CompleteAction {
  action: "Complete";
  result: string;
  reasoning: string;
  confidence: number;
}

interface DecomposeAction {
  action: "Decompose";
  subtasks: TaskStep[];
  execution_mode: "Sequential" | "Parallel";
  reasoning: string;
  confidence: number;
}

interface GatherContextAction {
  action: "GatherContext";
  needs: string[];
  reasoning: string;
  confidence: number;
}

type SelfDeterminationResult = CompleteAction | DecomposeAction | GatherContextAction;

// TaskStep for decomposition
interface TaskStep {
  description: string;
  difficulty: number;
}

// TaskList for multi-step workflows (from typescript_agent.md)
interface TaskList {
  id: string;
  title: string;
  steps: TaskListStep[];
  status?: "NotStarted" | "InProgress" | "Completed" | "Failed";
}

interface TaskListStep {
  id: string;
  step_type: "Debug" | "Feature" | "Refactor" | "Verify" | "Test";
  description: string;
  verification: string;
  status?: "Pending" | "InProgress" | "Completed" | "Failed";
  error?: string;
  result?: string;
  exit_command?: string | null;
}
