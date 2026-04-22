---
inclusion: auto
fileMatchPattern: "tasks.md"
---

# Task Execution Efficiency Rules

## Parallel Task Completion Strategy

When executing spec tasks, follow these efficiency rules to minimize token usage and execution time.

### 1. Batch Related Tasks
- Group tasks from the same phase or section (e.g., all Phase 2.1 tasks together)
- Complete 5-10 tasks per batch for optimal efficiency
- Prioritize tasks that can be completed together (same file, same component)

### 2. Use Parallel Tool Invocation

**CRITICAL**: When marking multiple independent tasks as complete, invoke taskStatus multiple times in a SINGLE function_calls block.

**Benefits:**
- Executes in parallel instead of sequentially
- Saves tokens (one block vs multiple blocks)
- Faster execution (no waiting between calls)
- More efficient workflow

**When to Use Parallel Invocation:**
- Marking multiple tasks complete after creating a single file
- Completing a group of related tasks in the same phase
- Any scenario where tasks have NO dependencies on each other

**When NOT to Use:**
- Tasks that depend on previous task completion
- Tasks that require reading output from previous tasks
- Tasks that modify the same file sequentially

### 3. Implementation Pattern

After creating a file that implements multiple tasks:
1. Identify all tasks completed by that file
2. Group them by phase (2.1, 2.2, 2.3, etc.)
3. Invoke taskStatus for each group in parallel
4. Maximum 8-10 invocations per function_calls block

### 4. Task Completion Order

While completing tasks in parallel, still respect logical order:
- Complete Phase 1 before Phase 2
- Complete parent tasks after all subtasks
- Mark tasks complete only when fully implemented

### 5. Error Handling

If a parallel batch fails:
- Review which tasks actually completed
- Retry failed tasks individually if needed
- Don't re-mark already completed tasks

## Example Workflow

1. Create StellarWalletContext.tsx (implements 23 tasks)
2. Batch 1: Mark Phase 2.1 tasks complete (8 parallel calls)
3. Batch 2: Mark Phase 2.2 tasks complete (8 parallel calls)
4. Batch 3: Mark Phase 2.3 tasks complete (7 parallel calls)
5. Result: 23 tasks completed in 3 batches instead of 23 sequential calls

## Token Savings

- Sequential: ~150 tokens per task × 23 = 3,450 tokens
- Parallel (3 batches): ~500 tokens per batch × 3 = 1,500 tokens
- **Savings: ~1,950 tokens (56% reduction)**
