# Git Worktrees for Parallel Agents

Isolate parallel Claude agents using git worktrees to prevent file conflicts.

## Problem

When spawning multiple Sonnet agents in parallel, they may:
- Edit the same file simultaneously
- Create conflicting commits
- Step on each other's changes

## Solution

Each agent works in its own git worktree:
- Same repository, different working directory
- Separate branch per task
- Merge back to main when complete

## Workflow

```
main repo: /Users/log/Projects/imbolc/
           ├── src/
           ├── docs/
           └── ...

worktree:  /Users/log/Projects/imbolc-worktrees/
           ├── task-48/     ← Agent 1 works here
           │   ├── src/
           │   └── ...
           ├── task-50/     ← Agent 2 works here
           │   ├── src/
           │   └── ...
           └── task-51/     ← Agent 3 works here
               ├── src/
               └── ...
```

## Implementation

### Creating Worktrees

```bash
# Create worktree directory
mkdir -p ../imbolc-worktrees

# Create worktree for a task
git worktree add ../imbolc-worktrees/task-48 -b task-48
git worktree add ../imbolc-worktrees/task-50 -b task-50
git worktree add ../imbolc-worktrees/task-51 -b task-51
```

### Agent Prompt Template

When spawning an agent:

```
You are working in a git worktree at: /Users/log/Projects/imbolc-worktrees/task-48

This is an isolated copy of the repository. You can freely edit files
without affecting other parallel agents.

When done:
1. Commit your changes to the task-48 branch
2. Do NOT merge - the orchestrator will handle that

Your task: [task description]
```

### Merging Back

After agents complete:

```bash
# Switch to main
cd /Users/log/Projects/imbolc
git checkout main

# Merge each task branch
git merge task-48 --no-ff -m "feat: Task 48 - free mixer channel on delete"
git merge task-50 --no-ff -m "feat: Task 50 - MixerViewRenderer"
git merge task-51 --no-ff -m "feat: Task 51 - MixerViewDispatcher"

# Handle any conflicts interactively

# Cleanup worktrees
git worktree remove ../imbolc-worktrees/task-48
git worktree remove ../imbolc-worktrees/task-50
git worktree remove ../imbolc-worktrees/task-51

# Delete branches
git branch -d task-48 task-50 task-51
```

### Conflict Resolution Strategy

If two tasks modify the same file:
1. Merge first task (fast-forward or clean merge)
2. Merge second task - conflict likely
3. Resolve manually or ask user
4. Continue with remaining tasks

### Orchestrator Pseudocode

```python
def run_parallel_tasks(tasks):
    worktrees = []

    # Create worktrees
    for task in tasks:
        branch = f"task-{task.id}"
        path = f"../imbolc-worktrees/{branch}"
        exec(f"git worktree add {path} -b {branch}")
        worktrees.append((task, path, branch))

    # Spawn agents in parallel
    agents = []
    for task, path, branch in worktrees:
        agent = spawn_agent(
            prompt=task.prompt,
            working_dir=path,
            branch=branch
        )
        agents.append(agent)

    # Wait for all agents
    results = await_all(agents)

    # Merge back sequentially
    exec("git checkout main")
    for task, path, branch in worktrees:
        try:
            exec(f"git merge {branch} --no-ff")
        except MergeConflict:
            notify_user(f"Conflict merging {branch}")
            # Manual resolution needed

    # Cleanup
    for task, path, branch in worktrees:
        exec(f"git worktree remove {path}")
        exec(f"git branch -d {branch}")
```

## Benefits

| Without Worktrees | With Worktrees |
|-------------------|----------------|
| File conflicts | Isolated changes |
| Race conditions | Independent branches |
| Partial commits | Clean history |
| Hard to debug | Easy to inspect |

## Limitations

- Disk space: Each worktree is a full checkout
- Merge conflicts still possible (but cleaner)
- More setup/teardown overhead
- Need to track which worktree each agent uses

## Future: Worktree Pool

Maintain a pool of pre-created worktrees:

```bash
# Pre-create pool
for i in {1..5}; do
    git worktree add ../imbolc-worktrees/pool-$i -b pool-$i
done

# Agent claims a worktree from pool
# Agent releases worktree when done (reset to main)
git -C ../imbolc-worktrees/pool-1 checkout main
git -C ../imbolc-worktrees/pool-1 reset --hard origin/main
```

This avoids create/remove overhead for each task.
