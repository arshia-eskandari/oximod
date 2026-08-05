//! Work with one document through its MongoDB object ID.
//!
//! Run with:
//!
//! ```text
//! cargo run -p oximod --example by_id
//! ```
//!
//! This example demonstrates how to:
//!
//! - save a model and receive its generated MongoDB `_id`;
//! - retrieve a typed model with `find_by_id()`;
//! - update one document with `update_by_id()`;
//! - inspect MongoDB's matched and modified counts;
//! - re-fetch a document after an update;
//! - delete a document with `delete_by_id()`;
//! - distinguish a missing document from an operation error.
//!
//! Set `MONGODB_URI` in the environment or in a `.env` file before running
//! the example.

use mongodb::bson::{doc, oid::ObjectId};
use oximod::{Model, OxiClient};
use serde::{Deserialize, Serialize};

// A support ticket stored as an independent MongoDB document.
#[derive(Debug, Serialize, Deserialize, Model)]
#[db("oximod_examples")]
#[collection("by_id_tickets")]
struct SupportTicket {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,

    #[validate(non_empty)]
    reference: String,

    #[validate(min_length = 5)]
    subject: String,

    /// New tickets begin in the open state.
    #[default("open".to_string())]
    status: String,

    assignee: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let mongodb_uri = std::env::var("MONGODB_URI")
        .expect("MONGODB_URI must be set in the environment or a .env file");

    OxiClient::init_global(mongodb_uri).await?;

    // This collection belongs only to the example, so clearing it keeps
    // repeated runs deterministic.
    SupportTicket::clear().await?;

    let ticket = SupportTicket::new()
        .reference("Ticket1")
        .subject("Customer cannot sign in");

    // `save()` validates and inserts the document. MongoDB generates the
    // object ID because `_id` was left as `None`.
    let ticket_id = ticket.save().await?;

    println!("Created Ticket1 with _id: {ticket_id}");

    // A valid ObjectId may not correspond to an existing document, so
    // `find_by_id()` returns `Option<SupportTicket>`.
    let saved_ticket = SupportTicket::find_by_id(ticket_id)
        .await?
        .expect("Ticket1 should exist immediately after being saved");

    println!(
        "Found ticket: {} | status: {} | assignee: {}",
        saved_ticket.reference,
        saved_ticket.status,
        display_assignee(&saved_ticket),
    );

    // `update_by_id()` accepts an ordinary MongoDB update document and returns
    // the driver's UpdateResult. Raw update documents are not passed through
    // the model's generated validation rules.
    let update_result = SupportTicket::update_by_id(
        ticket_id,
        doc! {
            "$set": {
                "status": "resolved",
                "assignee": "User2",
            },
        },
    )
    .await?;

    println!(
        "Update result: {} matched, {} modified",
        update_result.matched_count, update_result.modified_count
    );

    // The previously loaded Rust value is only a snapshot. Re-fetch the
    // document to observe changes made in MongoDB.
    let updated_ticket = SupportTicket::find_by_id(ticket_id)
        .await?
        .expect("Ticket1 should still exist after being updated");

    println!(
        "After update: {} | status: {} | assignee: {}",
        updated_ticket.reference,
        updated_ticket.status,
        display_assignee(&updated_ticket),
    );

    let delete_result = SupportTicket::delete_by_id(ticket_id).await?;

    println!(
        "Delete result: {} document deleted",
        delete_result.deleted_count
    );

    let ticket_after_deletion = SupportTicket::find_by_id(ticket_id).await?;

    match ticket_after_deletion {
        Some(ticket) => {
            println!("Unexpectedly found ticket: {}", ticket.reference);
        }
        None => {
            println!("Lookup after deletion: Ticket1 was not found");
        }
    }

    // Deleting the same ID again is not an error. MongoDB reports that no
    // document matched the operation.
    let second_delete = SupportTicket::delete_by_id(ticket_id).await?;

    println!(
        "Second deletion attempt: {} documents deleted",
        second_delete.deleted_count
    );

    Ok(())
}

fn display_assignee(ticket: &SupportTicket) -> &str {
    ticket.assignee.as_deref().unwrap_or("unassigned")
}
