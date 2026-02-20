# Basecamp API (To-dos Feature)

Reference: <https://github.com/basecamp/bc3-api>

## Core Difference

- `To-do set` (`Todoset`): the project-level container that owns top-level to-do lists.
- `To-do list` (`Todolist`): a standard list under a to-do set; can contain to-dos and groups.
- `To-do list group` (`Todolist`, grouped): a nested list-like section inside a to-do list used to organize items.
- `To-do` (`Todo`): the individual actionable item (task) inside a to-do list or group.

## Practical Hierarchy

```text
Project (bucket)
  -> To-do set (todoset)
    -> To-do lists (todolist, top-level)
      -> To-dos (todo)
      -> To-do list groups (also todolist type)
        -> To-dos (todo)
```

Notes:

- A group is represented as `Todolist` too, but it is nested under another to-do list.
- To-do list groups are read with the same endpoint shape as to-do lists (`GET /buckets/{project_id}/todolists/{id}.json`).
- Top-level lists expose `groups_url`; groups expose `group_position_url`.

## Endpoint Mapping

To-do sets:

- `GET /buckets/{project_id}/todosets/{todoset_id}.json`

To-do lists:

- `GET /buckets/{project_id}/todosets/{todoset_id}/todolists.json`
- `POST /buckets/{project_id}/todosets/{todoset_id}/todolists.json`
- `GET /buckets/{project_id}/todolists/{todolist_id}.json`

To-do list groups:

- `GET /buckets/{project_id}/todolists/{todolist_id}/groups.json`
- `POST /buckets/{project_id}/todolists/{todolist_id}/groups.json`
- `GET /buckets/{project_id}/todolists/{group_id}.json`
- `PUT /buckets/{project_id}/todolists/groups/{group_id}/position.json`

To-dos:

- `GET /buckets/{project_id}/todolists/{todolist_id}/todos.json`
- `POST /buckets/{project_id}/todolists/{todolist_id}/todos.json`
- `GET /buckets/{project_id}/todos/{todo_id}.json`
- `PUT /buckets/{project_id}/todos/{todo_id}.json`
- `POST /buckets/{project_id}/todos/{todo_id}/completion.json`
- `DELETE /buckets/{project_id}/todos/{todo_id}/completion.json`
- `TRASH /buckets/{project_id}/todos/{todo_id}.json`

Search:

- `GET /searches/metadata.json`
- `GET /search.json?q={query}&type=Todo`
- `GET /search.json?q={query}&type=Todo&bucket_id={project_id}`

Useful to-do fields/params for this CLI:

- `content` (required title)
- `description` (CLI: `--notes` or interactive `notes` prompt)
- `assignee_ids` (optional)
- `completion_subscriber_ids` (optional multi-user "When done, notify")
- `due_on` (CLI: `--due-on` or interactive prompt; optional `YYYY-MM-DD` date)
- `completed=true` (optional query param on list endpoint when fetching only completed items)
- `q` (required query string for `/search.json`)
- `type=Todo` (search filter for to-do results)
- `bucket_id` (optional project scope for `/search.json`)

## Implementation Guidance for This CLI

- Treat `todoset_id` as a required routing value discovered from the project `dock`.
- Treat both top-level lists and groups as list-like entities (`Todolist`) but keep separate CLI commands for clarity.
- Keep to-dos as the only task entity users can complete/uncomplete.
- For completion, use `POST /buckets/{project_id}/todos/{todo_id}/completion.json` (not `PUT /todos/{todo_id}.json`).
- Completion routing requires `project_id` and `todo_id` in the URL path; there is no project-agnostic complete endpoint.
- For text search completion UX, use account-level `GET /search.json` with `type=Todo`.
- To scope search to one project, include `bucket_id={project_id}` in `/search.json`.
- Map CLI `--notes` to Basecamp `description`, and CLI `--due-on` to Basecamp `due_on`.
- There is no documented fuzzy-search toggle/parameter; rely on `/search.json` query behavior and relevance ordering.
