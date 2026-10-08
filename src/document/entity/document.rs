use sea_orm::entity::prelude::*;

/// Document header. Totals (`total_base` … `total_czk`) are recomputed and
/// stored on every save; `paid` is the payment sum, kept in step by the
/// payment writes. Snapshots are filled at issue and never change afterwards.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "documents")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub direction: String,
    pub doc_type: String,
    pub status: String,
    pub number: Option<String>,
    pub number_year: Option<i32>,
    pub number_seq: Option<i32>,
    pub imported: bool,
    pub contact_id: Option<Uuid>,
    pub issue_date: Date,
    pub tax_point_date: Option<Date>,
    pub due_date: Date,
    pub currency: String,
    #[sea_orm(column_type = "Decimal(Some((18, 6)))", nullable)]
    pub exchange_rate: Option<Decimal>,
    pub exchange_rate_date: Option<Date>,
    pub exchange_rate_source: Option<String>,
    pub locale: String,
    pub vat_mode: String,
    pub bank_account_id: Option<Uuid>,
    pub payment_method: String,
    pub variable_symbol: Option<String>,
    pub constant_symbol: Option<String>,
    pub order_ref: Option<String>,
    pub header_note: Option<String>,
    pub footer_note: Option<String>,
    pub internal_note: Option<String>,
    pub round_total: bool,
    pub supplier_snapshot: Option<Json>,
    pub customer_snapshot: Option<Json>,
    pub bank_snapshot: Option<Json>,
    #[sea_orm(column_type = "Decimal(Some((18, 2)))")]
    pub total_base: Decimal,
    #[sea_orm(column_type = "Decimal(Some((18, 2)))")]
    pub total_vat: Decimal,
    #[sea_orm(column_type = "Decimal(Some((18, 2)))")]
    pub total: Decimal,
    #[sea_orm(column_type = "Decimal(Some((18, 2)))")]
    pub rounding: Decimal,
    #[sea_orm(column_type = "Decimal(Some((18, 2)))")]
    pub payable: Decimal,
    #[sea_orm(column_type = "Decimal(Some((18, 2)))", nullable)]
    pub total_czk: Option<Decimal>,
    #[sea_orm(column_type = "Decimal(Some((18, 2)))")]
    pub paid: Decimal,
    pub sent_at: Option<DateTimeWithTimeZone>,
    pub cancelled_at: Option<DateTimeWithTimeZone>,
    pub cancel_reason: Option<String>,
    pub related_document_id: Option<Uuid>,
    /// DDPP only: the proforma payment it documents.
    pub payment_id: Option<Uuid>,
    /// Credit notes only.
    pub correction_reason: Option<String>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
