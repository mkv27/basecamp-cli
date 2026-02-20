# CLI Contract (To-dos Feature)

This first stage defines only one command:

```bash
basecamp-cli todo add
basecamp-cli todo add "Title/content"
```

## Goal

Create a new Basecamp to-do with a guided interactive questionnaire, without requiring users to manually provide API IDs.

## Command Surface

```bash
basecamp-cli todo add [content] [--json]
```

Optional flags:

- `--json`: return machine-readable output after creation.

Positional args:

- `content` (optional): to-do title/content. If provided, skip the title prompt.

## `basecamp-cli todo add`

Purpose:

- Create a to-do in a selected project/list (or list group) using interactive prompts.

Preconditions:

- User is logged in (`basecamp-cli login` already completed).
- Integration credentials and session tokens are available locally.

Behavior:

1. Verify active auth session and selected account.
2. Fetch available projects (`buckets`) for current account.
3. Ask user to select `project`.
4. Resolve the selected project `todoset` from project dock (no manual `set` input required).
5. Fetch top-level to-do lists from that `todoset`.
6. Ask user to select target `to-do list`.
7. Ask whether to place the item in a list group:
   - `No` (create in selected list)
   - `Yes` (choose an existing group from that list)
8. Resolve task title (`content`):
   - use positional `content` if provided
   - otherwise ask title interactively
9. Ask optional task details:
   - `notes` (optional; sent as Basecamp `description` API field)
   - `assignee` (optional, from project people)
   - `when done, notify` (optional, multi-select from project people)
   - `due date` (optional, `YYYY-MM-DD`)
10. Create the to-do in the resolved list/group.
11. Print success output (human or JSON).

## Questionnaire (Prompt Order)

1. `Project`: select one project.
2. `To-do list`: select one list in that project.
3. `Use group?`: yes/no.
4. `Group` (only if yes): select one group.
5. `Title`: required if positional `content` is not provided.
6. `Notes`: optional.
7. `Assignee`: optional.
8. `When done, notify`: optional, multi-select.
9. `Due date`: optional.

## API Mapping

- Project selection source:
  - `GET /projects.json` (or equivalent account projects endpoint in current auth context)
- Todoset resolution:
  - project `dock` lookup, then `GET /buckets/{project_id}/todosets/{todoset_id}.json`
- Top-level lists:
  - `GET /buckets/{project_id}/todosets/{todoset_id}/todolists.json`
- Groups (optional):
  - `GET /buckets/{project_id}/todolists/{todolist_id}/groups.json`
- Create todo:
  - `POST /buckets/{project_id}/todolists/{target_todolist_id}/todos.json`

## Output

Human example:

```text
Created todo "Prepare launch notes" in project "Marketing Site" / list "Launch" (id: 987654321).
```

JSON example:

```json
{
  "ok": true,
  "project_id": 123456789,
  "todolist_id": 456789123,
  "todo_id": 987654321,
  "content": "Prepare launch notes"
}
```

## Exit Codes (To-dos Stage 1)

- `0`: success
- `1`: generic failure
- `2`: invalid input
- `3`: authentication/session missing or expired
- `4`: project/list/group not found or not accessible
- `5`: API request failed

## Non-Goals (This Stage)

- `todo list`/`todo update`/`todo complete` commands
- creating/deleting to-do lists or groups
- bulk todo creation
