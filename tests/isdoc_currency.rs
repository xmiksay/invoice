//! Foreign-currency ISDOC imports after the import: saving a received
//! document without changes keeps the supplier's CZK amounts.

mod common;

use axum::http::{Method, StatusCode};
use common::documents::{get_doc, set_company};
use common::isdoc::{confirm, fixture};
use common::{TestDb, call, router};
use serde_json::{Value, json};

/// The PUT body the received form sends back for `d`, `payable` overridden.
fn echo(d: &Value, payable: &str) -> Value {
    json!({
        "direction": "received", "docType": d["docType"], "supplierNumber": d["supplierNumber"],
        "contactId": d["contactId"], "issueDate": d["issueDate"], "taxPointDate": d["taxPointDate"],
        "receivedDate": d["receivedDate"], "dueDate": d["dueDate"], "currency": d["currency"],
        "exchangeRate": d["exchangeRate"], "vatMode": d["vatMode"], "vatRecap": d["vatRecap"],
        "rounding": d["rounding"], "payable": payable, "vatDeductible": d["vatDeductible"]
    })
}

#[tokio::test]
async fn unchanged_put_keeps_the_imported_czk_amounts() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    set_company(&app, true).await;
    // The supplier's CZK amounts differ from round2(x × rate) by a rounding.
    let xml = fixture("received_eur.isdoc")
        .replace(
            "<PayableAmount>24335.00</PayableAmount>",
            "<PayableAmount>24335.01</PayableAmount>",
        )
        .replace(
            "<DifferenceTaxableAmount>24335.00</DifferenceTaxableAmount>",
            "<DifferenceTaxableAmount>24334.99</DifferenceTaxableAmount>",
        );
    let results = confirm(
        &app,
        &[("eur.isdoc", xml.as_bytes())],
        json!({ "selected": ["eur.isdoc"] }),
    )
    .await;
    let id = results[0]["documentId"].as_str().expect("id").to_string();
    let doc = get_doc(&app, &id).await;
    assert_eq!(doc["totals"]["totalCzk"], "24335.01");
    assert_eq!(doc["totals"]["recap"][0]["baseCzk"], "24334.99");

    let uri = format!("/api/documents/{id}");
    let (status, saved) = call(&app, Method::PUT, &uri, Some(echo(&doc, "1000.00"))).await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    assert_eq!(
        saved["totals"]["totalCzk"], "24335.01",
        "a no-op save keeps it"
    );
    assert_eq!(saved["totals"]["recap"][0]["baseCzk"], "24334.99");

    // A changed amount recomputes from the rate, as for manual documents.
    let (status, changed) = call(&app, Method::PUT, &uri, Some(echo(&doc, "999.00"))).await;
    assert_eq!(status, StatusCode::OK, "{changed}");
    assert_eq!(changed["totals"]["totalCzk"], "24310.67");
    assert_eq!(changed["totals"]["recap"][0]["baseCzk"], "24335.00");
}
