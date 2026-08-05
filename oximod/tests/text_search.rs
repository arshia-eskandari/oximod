//! Integration tests for typed MongoDB text searches.
//!
//! The tests cover indexed terms, phrases, exclusions, typed filters, language,
//! case and diacritic sensitivity, combined options, and relevance sorting.

mod common;

use common::init;
use mongodb::bson::oid::ObjectId;
use oximod::{Model, Queryable, TextSearch};
use serde::{Deserialize, Serialize};
use testresult::TestResult;

// Run test:
// cargo nextest run typed_query_text_search_matches_indexed_terms
#[tokio::test]
async fn typed_query_text_search_matches_indexed_terms() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_text_search_matches_indexed_terms")]
    pub struct Article {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,

        #[index(text)]
        content: String,
    }

    Article::clear().await?;

    Article::default()
        .name("Article1")
        .content("Building typed queries with Rust and MongoDB")
        .save()
        .await?;

    Article::default()
        .name("Article2")
        .content("Designing responsive web interfaces")
        .save()
        .await?;

    Article::default()
        .name("Article3")
        .content("Using Rust for backend services")
        .save()
        .await?;

    let articles: Vec<Article> = Article::query()
        .text("rust")
        .sort_by(|article| article.name.asc())
        .all()
        .await?;

    let names = articles
        .iter()
        .map(|article| article.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(names, vec!["Article1", "Article3"]);

    Article::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_text_search_supports_phrases_and_excluded_terms
#[tokio::test]
async fn typed_query_text_search_supports_phrases_and_excluded_terms() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_text_search_supports_phrases_and_excluded_terms")]
    pub struct Article {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,

        #[index(text)]
        content: String,
    }

    Article::clear().await?;

    Article::default()
        .name("Article1")
        .content("A typed rust mongodb query reference")
        .save()
        .await?;

    Article::default()
        .name("Article2")
        .content("A typed rust mongodb beginner guide")
        .save()
        .await?;

    Article::default()
        .name("Article3")
        .content("A mongodb and rust typed query reference")
        .save()
        .await?;

    let articles: Vec<Article> = Article::query()
        .text("\"rust mongodb\" -beginner")
        .all()
        .await?;

    assert_eq!(articles.len(), 1);
    assert_eq!(articles[0].name, "Article1");

    Article::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_text_search_combines_with_typed_filter
#[tokio::test]
async fn typed_query_text_search_combines_with_typed_filter() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_text_search_combines_with_typed_filter")]
    pub struct Article {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,
        published: bool,

        #[index(text)]
        content: String,
    }

    Article::clear().await?;

    Article::default()
        .name("Article1")
        .published(true)
        .content("Rust backend development")
        .save()
        .await?;

    Article::default()
        .name("Article2")
        .published(false)
        .content("Rust database development")
        .save()
        .await?;

    Article::default()
        .name("Article3")
        .published(true)
        .content("Frontend interface development")
        .save()
        .await?;

    let articles: Vec<Article> = Article::query()
        .text("rust")
        .filter(|article| article.published.eq(true))
        .all()
        .await?;

    assert_eq!(articles.len(), 1);
    assert_eq!(articles[0].name, "Article1");

    Article::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_text_search_accepts_language
#[tokio::test]
async fn typed_query_text_search_accepts_language() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_text_search_accepts_language")]
    pub struct Article {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,

        #[index(text, default_language = "none")]
        content: String,
    }

    Article::clear().await?;

    Article::default()
        .name("Article1")
        .content("running distributed systems")
        .save()
        .await?;

    Article::default()
        .name("Article2")
        .content("building database systems")
        .save()
        .await?;

    let articles: Vec<Article> = Article::query()
        .text(TextSearch::new("running").language("none"))
        .all()
        .await?;

    assert_eq!(articles.len(), 1);
    assert_eq!(articles[0].name, "Article1");

    Article::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_text_search_can_be_case_sensitive
#[tokio::test]
async fn typed_query_text_search_can_be_case_sensitive() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_text_search_can_be_case_sensitive")]
    pub struct Article {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,

        #[index(text)]
        content: String,
    }

    Article::clear().await?;

    Article::default()
        .name("Article1")
        .content("Rust systems development")
        .save()
        .await?;

    Article::default()
        .name("Article2")
        .content("rust systems development")
        .save()
        .await?;

    let articles: Vec<Article> = Article::query()
        .text(TextSearch::new("Rust").case_sensitive(true))
        .all()
        .await?;

    assert_eq!(articles.len(), 1);
    assert_eq!(articles[0].name, "Article1");

    Article::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_text_search_can_be_diacritic_sensitive
#[tokio::test]
async fn typed_query_text_search_can_be_diacritic_sensitive() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_text_search_can_be_diacritic_sensitive")]
    pub struct Article {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,

        #[index(text, default_language = "none")]
        content: String,
    }

    Article::clear().await?;

    Article::default()
        .name("Article1")
        .content("A café near City1")
        .save()
        .await?;

    Article::default()
        .name("Article2")
        .content("A cafe near City2")
        .save()
        .await?;

    let articles: Vec<Article> = Article::query()
        .text(
            TextSearch::new("café")
                .language("none")
                .diacritic_sensitive(true),
        )
        .all()
        .await?;

    assert_eq!(articles.len(), 1);
    assert_eq!(articles[0].name, "Article1");

    Article::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_text_search_accepts_all_options
#[tokio::test]
async fn typed_query_text_search_accepts_all_options() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_text_search_accepts_all_options")]
    pub struct Article {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,

        #[index(text, default_language = "none")]
        content: String,
    }

    Article::clear().await?;

    Article::default()
        .name("Article1")
        .content("Café Rust development")
        .save()
        .await?;

    Article::default()
        .name("Article2")
        .content("Cafe rust development")
        .save()
        .await?;

    let articles: Vec<Article> = Article::query()
        .text(
            TextSearch::new("Café")
                .language("none")
                .case_sensitive(true)
                .diacritic_sensitive(true),
        )
        .all()
        .await?;

    assert_eq!(articles.len(), 1);
    assert_eq!(articles[0].name, "Article1");

    Article::clear().await?;

    Ok(())
}

// Run test:
// cargo nextest run typed_query_text_search_sorts_by_relevance
#[tokio::test]
async fn typed_query_text_search_sorts_by_relevance() -> TestResult {
    init().await?;

    #[derive(Model, Serialize, Deserialize, Debug, PartialEq)]
    #[db("test")]
    #[collection("typed_query_text_search_sorts_by_relevance")]
    pub struct Article {
        #[serde(skip_serializing_if = "Option::is_none")]
        _id: Option<ObjectId>,

        name: String,

        #[index(text)]
        content: String,
    }

    Article::clear().await?;

    Article::default()
        .name("Article1")
        .content("Rust MongoDB typed query development")
        .save()
        .await?;

    Article::default()
        .name("Article2")
        .content("Rust backend development")
        .save()
        .await?;

    Article::default()
        .name("Article3")
        .content("MongoDB database development")
        .save()
        .await?;

    let articles: Vec<Article> = Article::query()
        .text("rust mongodb")
        .sort_by_text_score()
        .all()
        .await?;

    assert_eq!(articles.len(), 3);
    assert_eq!(articles[0].name, "Article1");

    Article::clear().await?;

    Ok(())
}
