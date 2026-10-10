//! Read tools (`readOnlyHint`).

use rmcp::model::{CallToolResult, JsonObject};
use rmcp::{tool, tool_router};
use serde_json::{Value, json};

use super::args::{input, parse};
use super::result::{paging, pdf_url, respond, to_value, with};
use super::{InvoiceMcp, document_value, params};
use crate::app::AppState;
use crate::ares::handlers::lookup_ico;
use crate::catalog::handlers::dto::CatalogItem;
use crate::catalog::repo::items as catalog_repo;
use crate::contact::handlers::dto::{Contact, ContactList};
use crate::contact::repo::contacts as contact_repo;
use crate::document::handlers::compute_input::ComputeInput;
use crate::document::handlers::documents::{compute_totals, list_page};
use crate::document::handlers::dto::{DocumentSummary, ListQuery, Payment};
use crate::document::repo::payments as payment_repo;
use crate::error::AppError;
use crate::settings::doc_type::{ISSUED, RECEIVED};
use crate::settings::handlers::bank_accounts::BankAccount;
use crate::settings::handlers::company::Company;
use crate::settings::handlers::number_series::NumberSeries;
use crate::settings::handlers::vat_rates::VatRate;
use crate::settings::repo;
use crate::time::current_year;

#[tool_router(router = read_tools, vis = "pub(super)")]
impl InvoiceMcp {
    #[tool(
        description = "List documents of one direction (issued = our invoices, received = supplier invoices), newest first, with filters. Returns { items, total }; each item has a pdfUrl when a PDF exists.",
        input_schema = input::<params::ListDocuments>(),
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn list_documents(&self, args: JsonObject) -> CallToolResult {
        respond(list_documents(&self.state, args).await)
    }

    #[tool(
        description = "One document (issued or received) with lines, totals, VAT recap, payments and pdfUrl.",
        input_schema = input::<params::Id>(),
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn get_document(&self, args: JsonObject) -> CallToolResult {
        respond(get_document(&self.state, args).await)
    }

    #[tool(
        description = "Search the address book (customers and suppliers). Returns { items, total }.",
        input_schema = input::<params::ListContacts>(),
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn list_contacts(&self, args: JsonObject) -> CallToolResult {
        respond(list_contacts(&self.state, args).await)
    }

    #[tool(
        description = "One contact by id.",
        input_schema = input::<params::Id>(),
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn get_contact(&self, args: JsonObject) -> CallToolResult {
        respond(get_contact(&self.state, args).await)
    }

    #[tool(
        description = "Look up a Czech company in the public ARES register by IČO. Returns a contact draft (name, DIČ, address) to pass to create_contact; nothing is stored.",
        input_schema = input::<params::Ares>(),
        annotations(read_only_hint = true, open_world_hint = true)
    )]
    async fn lookup_ares(&self, args: JsonObject) -> CallToolResult {
        respond(lookup_ares(&self.state, args).await)
    }

    #[tool(
        description = "Active catalog items (reusable invoice lines with unit price and VAT rate). Returns { items }.",
        input_schema = input::<params::Catalog>(),
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn list_catalog_items(&self, args: JsonObject) -> CallToolResult {
        respond(list_catalog_items(&self.state, args).await)
    }

    #[tool(
        description = "What a draft needs: own company profile, bank accounts, active VAT rates and number series. Read only.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn get_settings(&self) -> CallToolResult {
        respond(get_settings(&self.state).await)
    }

    #[tool(
        description = "Compute line bases and totals (VAT recap, rounding, CZK amounts) of unsaved lines without storing anything. Same rules and validation as saving a draft. Amounts may be decimal strings or numbers.",
        input_schema = input::<ComputeInput>(),
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn compute_document(&self, args: JsonObject) -> CallToolResult {
        respond(compute_document(&self.state, args).await)
    }
}

fn summary_value(s: &DocumentSummary) -> Result<Value, AppError> {
    let url = pdf_url(s.id, &s.direction, s.imported, s.has_pdf);
    Ok(with(to_value(s)?, "pdfUrl", url))
}

async fn list_documents(state: &AppState, args: JsonObject) -> Result<Value, AppError> {
    let p: params::ListDocuments = parse(args)?;
    let direction = p.direction.trim();
    if direction != ISSUED && direction != RECEIVED {
        return Err(AppError::field("direction", "invalid"));
    }
    let (term, limit, offset) = paging(p.q, p.limit, p.offset);
    let query = ListQuery {
        direction: Some(direction.to_string()),
        doc_type: p.doc_type.map(|t| t.trim().to_string()),
        status: p.status,
        payment_state: p.payment_state,
        overdue: p.overdue,
        contact_id: p.contact_id,
        from: p.from,
        to: p.to,
        ..ListQuery::default()
    };
    let page = list_page(state, &query, term.as_deref(), limit, offset).await?;
    let items = page
        .items
        .iter()
        .map(summary_value)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(json!({ "items": items, "total": page.total }))
}

async fn get_document(state: &AppState, args: JsonObject) -> Result<Value, AppError> {
    let params::Id { id } = parse(args)?;
    let (doc, payments) =
        tokio::try_join!(document_value(state, id), payment_repo::list(&state.db, id))?;
    let payments: Vec<Payment> = payments.into_iter().map(Into::into).collect();
    Ok(with(doc, "payments", to_value(&payments)?))
}

async fn list_contacts(state: &AppState, args: JsonObject) -> Result<ContactList, AppError> {
    let p: params::ListContacts = parse(args)?;
    let (term, limit, offset) = paging(p.q, p.limit, p.offset);
    let (items, total) = contact_repo::list(&state.db, term.as_deref(), limit, offset).await?;
    Ok(ContactList {
        items: items.into_iter().map(Into::into).collect(),
        total,
    })
}

async fn get_contact(state: &AppState, args: JsonObject) -> Result<Contact, AppError> {
    let params::Id { id } = parse(args)?;
    Ok(contact_repo::get(&state.db, id).await?.into())
}

async fn lookup_ares(state: &AppState, args: JsonObject) -> Result<Value, AppError> {
    let p: params::Ares = parse(args)?;
    to_value(&lookup_ico(state, &p.ico).await?)
}

async fn list_catalog_items(
    state: &AppState,
    args: JsonObject,
) -> Result<Vec<CatalogItem>, AppError> {
    let p: params::Catalog = parse(args)?;
    let q = p.q.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let rows = catalog_repo::list(&state.db, q, Some(true)).await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

async fn compute_document(state: &AppState, args: JsonObject) -> Result<Value, AppError> {
    let input: ComputeInput = parse(args)?;
    to_value(&compute_totals(state, input).await?)
}

async fn get_settings(state: &AppState) -> Result<Value, AppError> {
    let db = &state.db;
    let (company, banks, rates, series) = tokio::try_join!(
        repo::company::get(db),
        repo::bank_accounts::list(db),
        repo::vat_rates::list(db),
        repo::number_series::list(db),
    )?;
    let year = current_year();
    let banks: Vec<BankAccount> = banks.into_iter().map(Into::into).collect();
    let rates: Vec<VatRate> = rates
        .into_iter()
        .filter(|r| r.active)
        .map(Into::into)
        .collect();
    let series: Vec<NumberSeries> = series
        .into_iter()
        .map(|s| NumberSeries::build(s, year))
        .collect();
    Ok(json!({
        "company": to_value(&Company::from(company))?,
        "bankAccounts": to_value(&banks)?,
        "vatRates": to_value(&rates)?,
        "numberSeries": to_value(&series)?,
    }))
}
