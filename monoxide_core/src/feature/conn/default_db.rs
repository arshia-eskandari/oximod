use std::{ backtrace::Backtrace, sync::{ Arc, OnceLock } };
use crate::{ error::conn_error::MongoDbError, Printable };

static DEFAULT_DB: OnceLock<Arc<String>> = OnceLock::new();

pub fn get_default_db() -> Result<Arc<String>, MongoDbError> {
    let default_db = DEFAULT_DB.get()
        .cloned()
        .ok_or_else(||
            MongoDbError::DefaultDbNotFound(
                "Failed to clone arc".to_string()
            ).attach_printables(
                Backtrace::capture(),
                Some("Ensure you call `set_defaut_db` before using `get_default_db`.")
            )
        )?;

    Ok(default_db)
}

pub fn set_default_db(default_db: String) -> Result<(), MongoDbError> {
    DEFAULT_DB.set(default_db.into()).map_err(|_|
        MongoDbError::SetDefaultDb(
            "DEFAULT_DB set method failed.".to_string()
        ).attach_printables(
            Backtrace::capture(),
            Some("Ensure `set_default_db` is only called once, or restart the application.")
        )
    )?;
    Ok(())
}
