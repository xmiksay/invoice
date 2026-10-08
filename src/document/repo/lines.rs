//! Line and VAT-recap rows ↔ [`LineData`] / computed totals.

use anyhow::Context as _;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use crate::document::compute::Totals;
use crate::document::entity::{document_line, vat_recap};
use crate::document::line::{AdvanceData, AdvanceRow, ItemData, LineData};
use crate::error::AppError;

pub fn to_data(m: document_line::Model) -> Result<LineData, AppError> {
    let missing = |f: &str| anyhow::anyhow!("line {} has no {f}", m.id);
    Ok(match m.kind.as_str() {
        "item" => LineData::Item(ItemData {
            quantity: m.quantity.ok_or_else(|| missing("quantity"))?,
            unit_price: m.unit_price.ok_or_else(|| missing("unit_price"))?,
            discount_pct: m.discount_pct.unwrap_or_default(),
            vat_rate: m.vat_rate.ok_or_else(|| missing("vat_rate"))?,
            unit: m.unit,
            description: m.description,
        }),
        "text" => LineData::Text {
            description: m.description,
        },
        "subtotal" => LineData::Subtotal {
            description: m.description,
            refs: m.refs.unwrap_or_default(),
            collapse: m.collapse,
        },
        "advance" => {
            let recap: Vec<AdvanceRow> = match m.advance_recap {
                Some(v) => serde_json::from_value(v).context("decode advance recap")?,
                None => Vec::new(),
            };
            LineData::Advance(AdvanceData {
                document_id: m
                    .advance_document_id
                    .ok_or_else(|| missing("advance_document_id"))?,
                description: m.description,
                recap,
            })
        }
        other => {
            Err(anyhow::anyhow!("unknown line kind {other:?}")).context("load document lines")?
        }
    })
}

fn to_active(
    document_id: Uuid,
    position: i32,
    line: &LineData,
) -> Result<document_line::ActiveModel, AppError> {
    let mut row = document_line::ActiveModel {
        id: Set(Uuid::new_v4()),
        document_id: Set(document_id),
        position: Set(position),
        kind: Set(line.kind().to_string()),
        quantity: Set(None),
        unit: Set(None),
        unit_price: Set(None),
        discount_pct: Set(None),
        vat_rate: Set(None),
        refs: Set(None),
        collapse: Set(false),
        advance_document_id: Set(None),
        advance_recap: Set(None),
        ..Default::default()
    };
    match line {
        LineData::Item(i) => {
            row.description = Set(i.description.clone());
            row.quantity = Set(Some(i.quantity));
            row.unit = Set(i.unit.clone());
            row.unit_price = Set(Some(i.unit_price));
            row.discount_pct = Set(Some(i.discount_pct));
            row.vat_rate = Set(Some(i.vat_rate));
        }
        LineData::Text { description } => row.description = Set(description.clone()),
        LineData::Subtotal {
            description,
            refs,
            collapse,
        } => {
            row.description = Set(description.clone());
            row.refs = Set(Some(refs.clone()));
            row.collapse = Set(*collapse);
        }
        LineData::Advance(a) => {
            row.description = Set(a.description.clone());
            row.advance_document_id = Set(Some(a.document_id));
            row.advance_recap = Set(Some(
                serde_json::to_value(&a.recap).context("encode advance recap")?,
            ));
        }
    }
    Ok(row)
}

/// Replace all lines of a document.
pub async fn replace<C: ConnectionTrait>(
    db: &C,
    document_id: Uuid,
    lines: &[LineData],
) -> Result<(), AppError> {
    document_line::Entity::delete_many()
        .filter(document_line::Column::DocumentId.eq(document_id))
        .exec(db)
        .await?;
    if lines.is_empty() {
        return Ok(());
    }
    let rows = lines
        .iter()
        .zip(1..)
        .map(|(l, p)| to_active(document_id, p, l))
        .collect::<Result<Vec<_>, _>>()?;
    document_line::Entity::insert_many(rows).exec(db).await?;
    Ok(())
}

/// Replace the stored VAT recap of a document.
pub async fn replace_recap<C: ConnectionTrait>(
    db: &C,
    document_id: Uuid,
    totals: &Totals,
) -> Result<(), AppError> {
    vat_recap::Entity::delete_many()
        .filter(vat_recap::Column::DocumentId.eq(document_id))
        .exec(db)
        .await?;
    if totals.recap.is_empty() {
        return Ok(());
    }
    let rows = totals.recap.iter().map(|r| vat_recap::ActiveModel {
        document_id: Set(document_id),
        vat_rate: Set(r.vat_rate),
        base: Set(r.base),
        vat: Set(r.vat),
        base_czk: Set(r.base_czk),
        vat_czk: Set(r.vat_czk),
    });
    vat_recap::Entity::insert_many(rows).exec(db).await?;
    Ok(())
}
