# CLI Contract (To-dos Feature)

This stage defines three commands:

```bash
basecamp-cli todo add
basecamp-cli todo add "Title/content"
basecamp-cli todo add "Title/content" --notes "Context" --due-on 2026-03-31
basecamp-cli todo complete "search text"
basecamp-cli todo complete "search text" --project-id <project_id>
basecamp-cli todo complete --id <todo_id> --project-id <project_id>
basecamp-cli todo re-open "search text"
basecamp-cli todo re-open "search text" --project-id <project_id>
basecamp-cli todo re-open --id <todo_id> --project-id <project_id>
```

## Goal

- Create a new Basecamp to-do with a guided interactive questionnaire.
- Complete existing to-do items either from text search + multi-select or by direct ID.
- Re-open existing completed to-do items either from text search + multi-select or by direct ID.

## Command Surface

```bash
basecamp-cli todo add [content] [--notes <text>] [--due-on <YYYY-MM-DD>] [--json]
basecamp-cli todo complete [query] [--id <todo_id>] [--project-id <project_id>] [--json]
basecamp-cli todo re-open [query] [--id <todo_id>] [--project-id <project_id>] [--json]
```

`todo add` optional flags:

- `--notes <text>`: set optional notes/description without prompting.
- `--due-on <YYYY-MM-DD>`: set optional due date without prompting.
- `--json`: return machine-readable output after creation.

`todo add` positional args:

- `content` (optional): to-do title/content. If provided, skip the title prompt.

`todo complete` optional flags:

- `--id <todo_id>`: complete one to-do directly (skips interactive match selection).
- `--project-id <project_id>`: scope search mode to one project; required with `--id` in direct mode.
- `--json`: return machine-readable output after completion.

`todo complete` positional args:

- `query` (optional): text used to filter matching to-do content in interactive search mode.

`todo re-open` optional flags:

- `--id <todo_id>`: re-open one completed to-do directly (skips interactive match selection).
- `--project-id <project_id>`: scope search mode to one project; required with `--id` in direct mode.
- `--json`: return machine-readable output after re-opening.

`todo re-open` positional args:

- `query` (optional): text used to filter matching completed to-do content in interactive search mode.

Validation rules:

- `--id` and positional `query` are mutually exclusive.
- If `--id` is provided, `--project-id` must also be provided.
- If `--id` is not provided, command runs search mode with interactive multi-select.
- In search mode, `query` is required by the API. If not passed positionally, prompt for it interactively.
- If `--due-on` is provided on `todo add`, it must be a valid `YYYY-MM-DD` calendar date.
- On `todo re-open`, `--id` and positional `query` are mutually exclusive.
- On `todo re-open`, `--project-id` is required when using `--id`.
- On `todo re-open`, if `--id` is not provided, command runs search mode with interactive multi-select.

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
   - `notes` (optional; from `--notes` when provided, otherwise prompt; sent as Basecamp `description` API field)
   - `assignee` (optional, from project people)
   - `when done, notify` (optional, multi-select from project people)
   - `due date` (optional; from `--due-on` when provided, otherwise prompt; `YYYY-MM-DD`)
10. Create the to-do in the resolved list/group.
11. Print success output (human or JSON).

## `basecamp-cli todo complete`

Purpose:

- Complete existing to-dos from either interactive text search or direct ID mode.

Preconditions:

- User is logged in (`basecamp-cli login` already completed).
- Integration credentials and session tokens are available locally.

Behavior:

Direct mode (`--id` + `--project-id`):

1. Validate that `--id` and `--project-id` are present.
2. Call completion endpoint for that specific to-do.
3. Print success output (human or JSON).

Search mode (default, without `--id`):

1. Verify active auth session and selected account.
2. Resolve search text:
   - use positional `query` if provided
   - otherwise ask query interactively
3. Search via Basecamp account search endpoint:
   - `GET /search.json?q={query}&type=Todo`
   - if `--project-id` is provided, include `bucket_id={project_id}` to scope search.
4. Show matching to-do results in an interactive multi-select list with project context and IDs.
5. Require at least one selected to-do.
6. Complete each selected to-do by calling the completion endpoint.
7. Print success summary (human or JSON).

## `basecamp-cli todo re-open`

Purpose:

- Re-open one completed to-do by ID.

Preconditions:

- User is logged in (`basecamp-cli login` already completed).
- Integration credentials and session tokens are available locally.

Behavior:

Direct mode (`--id` + `--project-id`):

1. Validate that `--id` and `--project-id` are present.
2. Call the re-open endpoint for that specific to-do.
3. Print success output (human or JSON).

Search mode (default, without `--id`):

1. Verify active auth session and selected account.
2. Resolve search text:
   - use positional `query` if provided
   - otherwise ask query interactively
3. Search via Basecamp account search endpoint:
   - `GET /search.json?q={query}&type=Todo`
   - if `--project-id` is provided, include `bucket_id={project_id}` to scope search.
4. Show matching completed to-do results in an interactive multi-select list with project context and IDs.
5. Require at least one selected to-do.
6. Re-open each selected to-do by calling the re-open endpoint.
7. Print success summary (human or JSON).

## Questionnaire (Prompt Order)

`todo add`:

1. `Project`: select one project.
2. `To-do list`: select one list in that project.
3. `Use group?`: yes/no.
4. `Group` (only if yes): select one group.
5. `Title`: required if positional `content` is not provided.
6. `Notes`: optional (prompt only when `--notes` is not provided).
7. `Assignee`: optional.
8. `When done, notify`: optional, multi-select.
9. `Due date`: optional (prompt only when `--due-on` is not provided).

`todo complete` (search mode):

1. `Search text` (only if positional `query` is not provided): enter text query.
2. `To-dos`: multi-select matching results to complete.

`todo re-open` (search mode):

1. `Search text` (only if positional `query` is not provided): enter text query.
2. `To-dos`: multi-select matching completed results to re-open.

## API Mapping

`todo add`:

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

`todo complete`:

- To-do search (account-wide):
  - `GET /search.json?q={query}&type=Todo`
- To-do search (scoped by project):
  - `GET /search.json?q={query}&type=Todo&bucket_id={project_id}`
- Complete to-do (direct + search modes):
  - `POST /buckets/{project_id}/todos/{todo_id}/completion.json`

`todo re-open`:

- To-do search (account-wide):
  - `GET /search.json?q={query}&type=Todo`
- To-do search (scoped by project):
  - `GET /search.json?q={query}&type=Todo&bucket_id={project_id}`
- Re-open completed to-do (direct + search modes):
  - `DELETE /buckets/{project_id}/todos/{todo_id}/completion.json`

## Output

`todo add` human example:

```text
Created todo "Prepare launch notes" in project "Marketing Site" / list "Launch" (id: 987654321).
```

`todo add` JSON example:

```json
{
  "ok": true,
  "project_id": 123456789,
  "todolist_id": 456789123,
  "todo_id": 987654321,
  "content": "Prepare launch notes"
}
```

`todo complete` human example:

```text
Completed 2 todos (987654321 in project 123456789, 987654322 in project 456789123).
```

`todo complete` JSON example:

```json
{
  "ok": true,
  "mode": "search",
  "completed": [
    { "todo_id": 987654321, "project_id": 123456789 },
    { "todo_id": 987654322, "project_id": 456789123 }
  ],
  "count": 2
}
```

`todo re-open` human example:

```text
Re-opened 2 todos (987654321 in project 123456789, 987654322 in project 456789123).
```

`todo re-open` JSON example:

```json
{
  "ok": true,
  "mode": "search",
  "reopened": [
    { "todo_id": 987654321, "project_id": 123456789 },
    { "todo_id": 987654322, "project_id": 456789123 }
  ],
  "count": 2
}
```

## Exit Codes (To-dos Stage 1)

- `0`: success
- `1`: generic failure
- `2`: invalid input
- `3`: authentication/session missing or expired
- `4`: project/list/group/todo not found or not accessible
- `5`: API request failed

## Non-Goals (This Stage)

- `todo list`/`todo update` commands
- creating/deleting to-do lists or groups
- bulk todo creation
