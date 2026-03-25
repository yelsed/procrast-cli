---
name: plan-idea
description: Browse your Procrast ideas, pick one, and automatically create an implementation plan for it. Use when the user wants to turn a captured idea into an actionable development plan.
---

# Plan Idea — Turn a Procrast idea into an implementation plan

## Workflow

### Step 1: List ideas

Call `list_ideas` with `limit: 50` and `hide_done: true` to fetch active ideas.

Present them as a numbered list:

```
# Your Procrast Ideas

1. [abc123] Build a habit tracker — priority: high
2. [def456] Add dark mode to the app — priority: medium
3. ...

Type a number to select, or tell me what to search for.
```

### Step 2: Search (if requested)

If the user provides a search term instead of a number, call `search_ideas` with their query and present matching results the same way.

### Step 3: Get full idea details

Once the user picks an idea (by number, UUID, or description), call `show_idea` with the UUID to get the complete idea including:
- Original content
- Refined content (if available)
- Action steps (if available)
- Priority and status

### Step 4: Enter plan mode

After retrieving the full idea, **immediately enter plan mode** using `EnterPlanMode`. Then:

1. Display the idea summary to confirm what you're planning
2. Explore the codebase to understand what already exists relevant to this idea
3. Create a detailed implementation plan that covers:
   - What needs to be built or changed
   - Which files to modify or create
   - The order of implementation steps
   - How to verify the changes work
4. If the idea has action steps from AI refinement, use those as a starting point but validate them against the actual codebase

## Important

- Always show ideas BEFORE asking the user to pick — don't ask them to type a UUID from memory
- If `check_auth` fails or `list_ideas` returns an error, tell the user to run `procrast login` in their terminal first
- Keep the idea list scannable — show title, priority, and a short UUID prefix only
- When entering plan mode, reference the idea's content directly so the plan is grounded in what the user captured
- Call `check_update` alongside listing ideas. If an update is available, mention it briefly to the user (e.g. "A CLI update is available — run `procrast update` to get the latest features.")
