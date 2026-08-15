# Lifecycle Hooks

Lifecycle hooks are optional. Enable them on a collection model with
`#[hooks]`, then implement `Hooks`:

```rust
use oximod::{Hooks, Model, OxiModError};

#[derive(Debug, Serialize, Deserialize, Model)]
#[db("app")]
#[collection("users")]
#[hooks]
struct User {
    email: String,
    name: String,
}

#[async_trait::async_trait]
impl Hooks for User {
    async fn pre_save_mut(
        &mut self,
    ) -> Result<(), OxiModError> {
        self.email = self.email.trim().to_lowercase();
        self.name = self.name.trim().to_string();
        Ok(())
    }
}
```

Every hook has a default no-op implementation. Override only the events the
model needs.

## Save hooks

| Hook            | Runs for                                             | Behavior                                                                             |
| --------------- | ---------------------------------------------------- | ------------------------------------------------------------------------------------ |
| `pre_save`      | `save`, `save_from`, `save_with_session`             | Immutable check before validation and insertion.                                     |
| `post_save`     | `save`, `save_from`, `save_with_session`             | Runs after insertion.                                                                |
| `pre_save_mut`  | `save_mut`, `save_from_mut`, `save_mut_with_session` | May mutate the model before validation and insertion.                                |
| `post_save_mut` | `save_mut`, `save_from_mut`, `save_mut_with_session` | May mutate in-memory state after insertion; changes are not automatically persisted. |

Each save form runs only its own hooks: `save()`-family methods run
`pre_save`/`post_save`, while `save_mut()`-family methods run
`pre_save_mut`/`post_save_mut`. A safeguard implemented in only one pre-save
hook does not guard the other save form; implement both when the application
uses both.

## `_id` helper hooks

| Hook                         | Runs for                                                          |
| ---------------------------- | ----------------------------------------------------------------- |
| `pre_find` / `post_find`     | `find_by_id`, `find_by_id_from`, `find_by_id_with_session`        |
| `pre_update` / `post_update` | `update_by_id`, `update_by_id_from`, `update_by_id_with_session`  |
| `pre_delete` / `post_delete` | `delete_by_id`, `delete_by_id_from`, `delete_by_id_with_session`  |

## Hook boundaries

Hooks do **not** wrap:

* typed-query reads, updates, or deletions;
* direct typed or raw collection operations;
* bulk-write operations;
* `clear`;
* `exists`;
* `count`;
* collection accessors.

A pre-hook error prevents the associated database operation. A post-hook error
is returned after the database operation has already succeeded.

## Hooks and sessions

Hook callbacks never receive a `ClientSession`. When a `_with_session` helper
fires hooks, a database operation initiated inside a hook executes without the
session and is therefore **not** part of the caller's transaction; perform
transactional side writes explicitly in the transaction body instead. A
post-hook for a session-aware helper runs after that MongoDB operation
succeeded in the session — not after the transaction committed — so an aborted
transaction rolls back the operation even though its post-hook already ran.

## Related material

* Runnable workflow: `hook_usage` in
  [Runnable Examples](../reference/examples.md).
