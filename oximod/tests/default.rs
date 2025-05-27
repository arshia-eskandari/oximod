use mongodb::bson::{ doc, oid::ObjectId };
use oximod::Model;
use testresult::TestResult;
use serde::{ Deserialize, Serialize };

mod common;
use common::init;

// Run test: cargo nextest run saves_with_default_string_and_number
#[tokio::test]
async fn saves_with_default_string_and_number() -> TestResult {
    init().await;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("defaults_str_num")]
    pub struct Thing {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[default("Anonymous".to_string())]
        name: String,

        #[default(0)]
        count: i32,
    }

    Thing::clear().await?;

    // We don't call `.name(...)` or `.count(...)` so defaults should apply:
    let thing = Thing::default();
    let id = thing.save().await?;

    // Fetch raw document to inspect defaults:
    let doc = Thing::find_one(doc! { "_id": id.clone() }).await?
        .unwrap();

    assert_eq!(doc.name, "Anonymous");
    assert_eq!(doc.count, 0);

    Ok(())
}

// Run test: cargo nextest run override_default_values
#[tokio::test]
async fn override_default_values() -> TestResult {
    init().await;

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("defaults_override")]
    pub struct Record {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[default("Guest".to_string())]
        user: String,

        #[default(1)]
        retries: i32,
    }

    Record::clear().await?;

    // Override both defaults via fluent API:
    let rec = Record::default().user("Alice".to_string()).retries(5);
    let id = rec.save().await?;

    let got = Record::find_by_id(id).await?.unwrap();
    assert_eq!(got.user, "Alice");
    assert_eq!(got.retries, 5);

    Ok(())
}

// Run test: cargo nextest run enum_default_and_override
#[tokio::test]
async fn enum_default_and_override() -> TestResult {
    init().await;

    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
    pub enum Status {
        Pending,
        Complete,
    }

    #[derive(Model, Serialize, Deserialize, Debug)]
    #[db("test")]
    #[collection("defaults_enum")]
    pub struct Task {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        #[default(Status::Pending)]
        status: Status,

        description: String,
    }

    Task::clear().await?;

    // Without overriding, status == Pending
    let t1 = Task::default().description("T1".into());
    let id1 = t1.save().await?;
    let got1 = Task::find_by_id(id1).await?.unwrap();
    assert_eq!(got1.status, Status::Pending);
    assert_eq!(got1.description, "T1");

    // Override to Complete
    let t2 = Task::default().status(Status::Complete).description("T2".into());
    let id2 = t2.save().await?;
    let got2 = Task::find_by_id(id2).await?.unwrap();
    assert_eq!(got2.status, Status::Complete);
    assert_eq!(got2.description, "T2");

    Ok(())
}
