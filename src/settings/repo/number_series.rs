use anyhow::Context;
use sea_orm::sea_query::{Expr, OnConflict};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DatabaseTransaction,
    EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set, Statement, TransactionTrait,
};

use crate::error::AppError;
use crate::settings::doc_type::DocType;
use crate::settings::entity::{number_series, number_series_counter as counter};
use crate::settings::pattern::Pattern;

/// A series with its counters, newest year first.
pub struct Series {
    pub doc_type: DocType,
    pub pattern: String,
    pub counters: Vec<counter::Model>,
}

pub async fn list(db: &DatabaseConnection) -> Result<Vec<Series>, AppError> {
    let series = number_series::Entity::find().all(db).await?;
    let counters = counter::Entity::find()
        .order_by_desc(counter::Column::Year)
        .all(db)
        .await?;
    DocType::ALL
        .into_iter()
        .map(|doc_type| {
            let pattern = series
                .iter()
                .find(|s| s.doc_type == doc_type.as_str())
                .map(|s| s.pattern.clone())
                .with_context(|| format!("number series {} missing", doc_type.as_str()))?;
            let counters = counters
                .iter()
                .filter(|c| c.doc_type == doc_type.as_str())
                .cloned()
                .collect();
            Ok(Series {
                doc_type,
                pattern,
                counters,
            })
        })
        .collect()
}

pub async fn get<C: ConnectionTrait>(db: &C, doc_type: DocType) -> Result<Series, AppError> {
    let pattern = load_pattern(db, doc_type).await?;
    let counters = counter::Entity::find()
        .filter(counter::Column::DocType.eq(doc_type.as_str()))
        .order_by_desc(counter::Column::Year)
        .all(db)
        .await?;
    Ok(Series {
        doc_type,
        pattern,
        counters,
    })
}

/// `pattern` must already be validated. Two doc types sharing a pattern would
/// issue identical numbers from separate counters → `pattern: duplicate`.
pub async fn set_pattern(
    db: &DatabaseConnection,
    doc_type: DocType,
    pattern: String,
) -> Result<Series, AppError> {
    let taken = number_series::Entity::find()
        .filter(number_series::Column::DocType.ne(doc_type.as_str()))
        .filter(number_series::Column::Pattern.eq(pattern.as_str()))
        .one(db)
        .await?;
    if taken.is_some() {
        return Err(AppError::field("pattern", "duplicate"));
    }
    number_series::ActiveModel {
        doc_type: Set(doc_type.as_str().to_string()),
        pattern: Set(pattern),
    }
    .update(db)
    .await?;
    get(db, doc_type).await
}

/// Manually set the last allocated number of a year (upsert). Going below
/// the highest sequence already issued (imported documents excluded) would
/// make the series hand out a duplicate number → `lastNumber: below_issued`.
pub async fn set_counter(
    db: &DatabaseConnection,
    doc_type: DocType,
    year: i32,
    last_number: i32,
) -> Result<Series, AppError> {
    db.transaction(|txn| Box::pin(set_counter_in(txn, doc_type, year, last_number)))
        .await?;
    get(db, doc_type).await
}

async fn set_counter_in(
    txn: &DatabaseTransaction,
    doc_type: DocType,
    year: i32,
    last_number: i32,
) -> Result<(), AppError> {
    // Lock the counter row before reading the issued numbers: an issue in
    // flight holds this row (via `allocate_number`'s upsert) until it commits,
    // so the guard waits for it and then sees its number — no stale check.
    counter::Entity::insert(counter::ActiveModel {
        doc_type: Set(doc_type.as_str().to_string()),
        year: Set(year),
        last_number: Set(0),
    })
    .on_conflict(
        OnConflict::columns([counter::Column::DocType, counter::Column::Year])
            .do_nothing()
            .to_owned(),
    )
    .do_nothing()
    .exec(txn)
    .await?;
    counter::Entity::find_by_id((doc_type.as_str().to_string(), year))
        .lock_exclusive()
        .one(txn)
        .await?
        .context("counter row missing right after its upsert")?;
    let highest = crate::document::repo::query::highest_issued_seq(txn, doc_type, year).await?;
    if highest.is_some_and(|h| last_number < h) {
        return Err(AppError::field("lastNumber", "below_issued"));
    }
    counter::Entity::update_many()
        .col_expr(counter::Column::LastNumber, Expr::value(last_number))
        .filter(counter::Column::DocType.eq(doc_type.as_str()))
        .filter(counter::Column::Year.eq(year))
        .exec(txn)
        .await?;
    Ok(())
}

/// Allocate the next number of `doc_type` in `year`; returns the rendered
/// number and its sequence.
///
/// Must run inside the transaction that persists the numbered document, so a
/// rollback also returns the number. The atomic `INSERT … ON CONFLICT DO
/// UPDATE … RETURNING` row-locks the counter: a concurrent allocation blocks
/// until this transaction ends, so numbers are distinct and gap-free.
pub async fn allocate_number(
    txn: &DatabaseTransaction,
    doc_type: DocType,
    year: i32,
) -> Result<(String, i32), AppError> {
    let pattern = load_pattern(txn, doc_type).await?;
    let pattern = Pattern::parse(&pattern)
        .with_context(|| format!("stored pattern of {} is invalid", doc_type.as_str()))?;
    let row = txn
        .query_one(Statement::from_sql_and_values(
            txn.get_database_backend(),
            "INSERT INTO number_series_counters (doc_type, year, last_number) VALUES ($1, $2, 1) \
             ON CONFLICT (doc_type, year) \
             DO UPDATE SET last_number = number_series_counters.last_number + 1 \
             RETURNING last_number",
            [doc_type.as_str().into(), year.into()],
        ))
        .await?
        .context("counter upsert returned no row")?;
    let n: i32 = row.try_get("", "last_number")?;
    Ok((pattern.format(year, i64::from(n)), n))
}

async fn load_pattern<C: ConnectionTrait>(db: &C, doc_type: DocType) -> Result<String, AppError> {
    let row = number_series::Entity::find_by_id(doc_type.as_str())
        .one(db)
        .await?
        .with_context(|| format!("number series {} missing", doc_type.as_str()))?;
    Ok(row.pattern)
}
