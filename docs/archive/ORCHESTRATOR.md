# Orchestrator Workflow

Guide for Opus to orchestrate parallel Sonnet agents using git worktrees.

## Overview

```
Opus (Orchestrator)
  ├── Creates worktrees
  ├── Spawns Sonnet agents in parallel
  ├── Monitors for completion
  ├── Merges immediately when each completes
  └── Cleans up worktrees
```

## Workflow

### 1. Select Non-Conflicting Tasks

Pick tasks that touch different files to avoid merge conflicts:

| Good Combinations | Why |
|-------------------|-----|
| src/state/ + src/core/ + src/panes/ | Different directories |
| New modules + tests | Isolated changes |
| Widgets + docs | Independent areas |

Avoid assigning tasks that modify the same files (e.g., two tasks both editing main.rs).

### 2. Create Worktrees

```bash
mkdir -p ../imbolc-worktrees
git worktree add ../imbolc-worktrees/task-N -b task-N
```

Create all worktrees upfront before spawning agents.

### 3. Spawn Agents in Parallel

Use Task tool with `run_in_background: true` for all agents simultaneously:

```
Task(
  subagent_type: "general-purpose",
  model: "sonnet",
  run_in_background: true,
  prompt: """
    You are working in a git worktree at: /path/to/imbolc-worktrees/task-N

    This is an isolated copy on branch task-N. You can freely edit files.

    ## Your Task: [description]

    [detailed requirements]

    ## Important:
    - Work ONLY in your worktree directory
    - Commit to your task-N branch
    - Do NOT merge to main - the orchestrator handles that
    - Use commit message format:
      ```
      <type>: <description>

      Refs: TASKS.md#task-N
      Co-Authored-By: Claude Sonnet <noreply@anthropic.com>
      ```
  """
)
```

### 4. Monitor and Merge on Completion

Use `TaskOutput` with `block: false` to poll for completion, or wait for task notifications.

**When a task completes, immediately:**

```bash
# Merge to main
git merge task-N --no-ff -m "feat: <description>

Refs: TASKS.md#task-N
Co-Authored-By: Claude Sonnet <noreply@anthropic.com>"

# Cleanup
git worktree remove ../imbolc-worktrees/task-N
git branch -d task-N
```

This prevents stale worktrees and keeps main up-to-date for other merges.

### 5. Handle Conflicts

If a merge conflicts:

1. Notify user with conflicting files
2. Options:
   - User resolves manually
   - Spawn agent to resolve (give it both versions)
   - Abort and reassign task

## Example Session

```
User: "Run 5 tasks in parallel"

Opus:
1. Creates 5 worktrees (task-1, task-2, task-3, task-4, task-5)
2. Spawns 5 Sonnet agents with run_in_background: true
3. Displays status table:
   | Agent | Task | Branch | Status |
   |-------|------|--------|--------|
   | 1 | State types | task-1 | Running |
   | 2 | Action enum | task-2 | Running |
   ...

4. When task-1 completes:
   - git merge task-1 --no-ff
   - git worktree remove ../imbolc-worktrees/task-1
   - git branch -d task-1
   - Update status: task-1 ✓ Merged

5. Repeat for each completion

6. Final summary with all commits
```

## Agent Prompt Template

```markdown
You are working in a git worktree at: {worktree_path}

This is an isolated copy of the repository on branch {branch_name}.
You can freely edit files without affecting other parallel agents.

## Your Task: {task_title} (Task {task_number})

From TASKS.md:
{task_description}

## Requirements:
{requirements}

## Steps:
1. Read relevant existing code to understand patterns
2. Implement the feature following existing conventions
3. Run `cargo build` and `cargo test` to verify
4. Commit your changes

## Important:
- Work ONLY in {worktree_path}
- Commit to your {branch_name} branch
- Do NOT merge to main - the orchestrator handles that
- Do NOT push - orchestrator handles that
- Use commit message format:
  ```
  {type}: {short_description}

  Refs: TASKS.md#task-{task_number}
  Co-Authored-By: Claude Sonnet <noreply@anthropic.com>
  ```
```

## Worker Pool Mode

Instead of running fixed batches, maintain a continuous pool of N workers (typically 3-5).
As each worker completes, immediately merge and spawn a new task.

### Pool Lifecycle

```
┌─────────────────────────────────────────────────────────────┐
│                    Worker Pool (N=3)                        │
├─────────────────────────────────────────────────────────────┤
│  Slot 1: task-1 ████████░░ 80%                             │
│  Slot 2: task-2 ██████████ ✓ → merge → task-4 ██░░         │
│  Slot 3: task-3 ███░░░░░░░ 30%                             │
└─────────────────────────────────────────────────────────────┘
```

### Implementation

1. **Initialize pool** with N tasks:
   ```bash
   # Create N worktrees
   for task in 1 2 3; do
     git worktree add ../imbolc-worktrees/task-$task -b task-$task
   done
   ```

2. **Spawn all N agents** in parallel with `run_in_background: true`

3. **Monitor with TaskOutput** - poll periodically:
   ```
   TaskOutput(task_id: "agent-1", block: false)
   TaskOutput(task_id: "agent-2", block: false)
   ...
   ```

4. **On completion**, immediately:
   - Merge the completed branch
   - Clean up worktree and branch
   - Pick next task from queue
   - Create new worktree
   - Spawn new agent in that slot

5. **Repeat** until task queue is empty

### Advantages over Batches

| Batches | Worker Pool |
|---------|-------------|
| Wait for slowest in batch | Fast tasks don't block |
| Fixed batch boundaries | Continuous throughput |
| Simple to reason about | Better resource utilization |
| All-or-nothing progress | Incremental progress |

### Task Queue Management

Maintain a priority queue of remaining tasks:
```
Queue: [4, 5, 6, 7, ...]
       ↑
       Next task to assign when a slot frees
```

Select next task considering:
1. **Dependencies**: Some tasks depend on others completing first
2. **Conflicts**: Don't assign if it touches files currently being edited
3. **Priority**: Foundation > Features > Polish

## Troubleshooting

| Issue | Solution |
|-------|----------|
| Merge conflict | Check which task modified unexpectedly, resolve manually |
| Agent stuck | Check output file, resume if needed with agent ID |
| Worktree locked | `git worktree prune` to clean stale locks |
| Branch exists | Delete old branch: `git branch -D task-N` |
| Pool starvation | Ensure task queue has enough non-conflicting tasks |
| Build fails after merge | Run `cargo build`, fix issues, commit fix |
