//! Categories, custom field definitions, document metadata.

mod common;

use axum::http::{Method, StatusCode};
use common::documents::{create_doc, create_issued, id, issuable, issue};
use common::received::{create_category, create_field, create_received};
use common::{TestDb, call, router};
use serde_json::{Value, json};

#[tokio::test]
async fn categories_crud() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    let sw = create_category(&app, "Software", "expense").await;
    create_category(&app, "Konzultace", "income").await;
    // Same name, other kind: fine; same kind (any case): duplicate.
    create_category(&app, "software", "income").await;
    let (status, e) = call(
        &app,
        Method::POST,
        "/api/settings/categories",
        Some(json!({ "name": "SOFTWARE", "kind": "expense" })),
    )
    .await;
    assert_eq!(
        (status, &e["fields"]["name"]),
        (StatusCode::UNPROCESSABLE_ENTITY, &json!("duplicate"))
    );
    let (_, e) = call(
        &app,
        Method::POST,
        "/api/settings/categories",
        Some(json!({ "name": "", "kind": "cost" })),
    )
    .await;
    assert_eq!(
        e["fields"],
        json!({ "name": "required", "kind": "invalid" })
    );
    let (_, list) = call(&app, Method::GET, "/api/settings/categories", None).await;
    let names: Vec<_> = list
        .as_array()
        .expect("array")
        .iter()
        .map(|c| (c["kind"].clone(), c["name"].clone()))
        .collect();
    assert_eq!(
        names,
        vec![
            (json!("expense"), json!("Software")),
            (json!("income"), json!("Konzultace")),
            (json!("income"), json!("software")),
        ]
    );
    let (status, c) = call(
        &app,
        Method::PUT,
        &format!("/api/settings/categories/{sw}"),
        Some(json!({ "name": "Software a licence", "kind": "expense", "position": 3 })),
    )
    .await;
    assert_eq!(
        (status, &c["name"], &c["position"]),
        (StatusCode::OK, &json!("Software a licence"), &json!(3))
    );

    // In use: no delete, no kind change; deactivate instead.
    let s = common::documents::create_contact(&app, json!({ "ico": "27074358" })).await;
    create_received(&app, &s, json!({ "categoryId": sw })).await;
    let uri = format!("/api/settings/categories/{sw}");
    let (status, e) = call(&app, Method::DELETE, &uri, None).await;
    assert_eq!(
        (status, &e["code"]),
        (StatusCode::CONFLICT, &json!("category_in_use"))
    );
    let (_, e) = call(
        &app,
        Method::PUT,
        &uri,
        Some(json!({ "name": "Software", "kind": "income" })),
    )
    .await;
    assert_eq!(e["fields"]["kind"], "invalid");
    let (status, c) = call(
        &app,
        Method::PUT,
        &uri,
        Some(json!({ "name": "Software", "kind": "expense", "active": false })),
    )
    .await;
    assert_eq!((status, &c["active"]), (StatusCode::OK, &json!(false)));
    // Inactive: kept where present, not newly assignable.
    let (status, e) = call(
        &app,
        Method::POST,
        "/api/documents",
        Some(common::received::received_body(
            &s,
            json!({ "categoryId": sw }),
        )),
    )
    .await;
    assert_eq!(
        (status, &e["fields"]["categoryId"]),
        (StatusCode::UNPROCESSABLE_ENTITY, &json!("inactive"))
    );
    let unused = create_category(&app, "Cestovné", "expense").await;
    let (status, _) = call(
        &app,
        Method::DELETE,
        &format!("/api/settings/categories/{unused}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(
        &app,
        Method::DELETE,
        &format!("/api/settings/categories/{unused}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn category_kind_per_direction() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    let mut body = issuable(&app).await;
    let expense = create_category(&app, "Nákup", "expense").await;
    let income = create_category(&app, "Tržby", "income").await;
    body["categoryId"] = json!(expense);
    let (_, e) = call(&app, Method::POST, "/api/documents", Some(body.clone())).await;
    assert_eq!(e["fields"]["categoryId"], "invalid");
    body["categoryId"] = json!(income);
    let doc = create_doc(&app, body).await;
    assert_eq!(doc["categoryId"], json!(income));
}

#[tokio::test]
async fn custom_fields_crud_and_validation() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    let project = create_field(
        &app,
        json!({ "key": "project", "label": "Projekt", "type": "text", "appliesTo": "issued", "required": true }),
    )
    .await;
    assert_eq!(
        (&project["type"], &project["active"], &project["options"]),
        (&json!("text"), &json!(true), &json!([]))
    );
    create_field(&app, json!({ "key": "channel", "label": "Kanál", "type": "select", "options": ["web", "email"] })).await;
    create_field(&app, json!({ "key": "cost_center", "label": "Středisko", "type": "number", "appliesTo": "received" })).await;
    let (status, e) = call(
        &app,
        Method::POST,
        "/api/settings/custom-fields",
        Some(json!({ "key": "project", "label": "X", "type": "text" })),
    )
    .await;
    assert_eq!(
        (status, &e["fields"]["key"]),
        (StatusCode::UNPROCESSABLE_ENTITY, &json!("duplicate"))
    );
    let uri = format!(
        "/api/settings/custom-fields/{}",
        project["id"].as_str().expect("id")
    );
    let (_, e) = call(
        &app,
        Method::PUT,
        &uri,
        Some(json!({ "key": "project2", "label": "P", "type": "bool" })),
    )
    .await;
    assert_eq!(e["fields"], json!({ "key": "invalid", "type": "invalid" }));
    let (_, list) = call(&app, Method::GET, "/api/settings/custom-fields", None).await;
    assert_eq!(list.as_array().expect("array").len(), 3);

    let mut body = issuable(&app).await;
    let (_, e) = call(&app, Method::POST, "/api/documents", Some(body.clone())).await;
    assert_eq!(e["fields"], json!({ "customFields.project": "required" }));
    body["customFields"] =
        json!({ "project": "Alfa", "channel": "fax", "cost_center": "1", "nope": 1 });
    let (_, e) = call(&app, Method::POST, "/api/documents", Some(body.clone())).await;
    assert_eq!(
        e["fields"],
        json!({ "customFields.channel": "invalid", "customFields.cost_center": "unknown", "customFields.nope": "unknown" })
    );
    body["customFields"] = json!({ "project": " Alfa ", "channel": "web" });
    let draft = create_doc(&app, body).await;
    assert_eq!(
        draft["customFields"],
        json!({ "project": "Alfa", "channel": "web" })
    );

    // A required field added after the save is enforced at issue.
    create_field(
        &app,
        json!({ "key": "approved", "label": "Schváleno", "type": "bool",
        "appliesTo": "issued", "required": true }),
    )
    .await;
    let (status, e) = issue(&app, &id(&draft)).await;
    assert_eq!(
        (status, &e["fields"]),
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            &json!({ "customFields.approved": "required" })
        )
    );

    // Deleting a definition keeps stored values (sent back unchanged).
    let (status, _) = call(&app, Method::DELETE, &uri, None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(&app, Method::DELETE, &uri, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let meta = format!("/api/documents/{}/metadata", id(&draft));
    let fields = json!({ "project": "Alfa", "channel": "web", "approved": false });
    let (status, d) = call(
        &app,
        Method::PUT,
        &meta,
        Some(json!({ "customFields": fields })),
    )
    .await;
    assert_eq!((status, &d["customFields"]), (StatusCode::OK, &fields));
    let changed = json!({ "project": "Beta", "channel": "web", "approved": false });
    let (_, e) = call(
        &app,
        Method::PUT,
        &meta,
        Some(json!({ "customFields": changed })),
    )
    .await;
    assert_eq!(e["fields"], json!({ "customFields.project": "unknown" }));
}

#[tokio::test]
async fn metadata_after_issue() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    let doc = create_issued(&app, issuable(&app).await).await;
    let cat = create_category(&app, "Tržby", "income").await;
    create_field(
        &app,
        json!({ "key": "flag", "label": "Flag", "type": "bool" }),
    )
    .await;
    let uri = format!("/api/documents/{}/metadata", id(&doc));
    let (status, d) = call(
        &app,
        Method::PUT,
        &uri,
        Some(json!({ "categoryId": cat, "customFields": { "flag": true }, "internalNote": "ok" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{d}");
    assert_eq!(
        (
            &d["categoryId"],
            &d["customFields"],
            &d["internalNote"],
            &d["status"]
        ),
        (
            &json!(cat),
            &json!({ "flag": true }),
            &json!("ok"),
            &json!("issued")
        )
    );
    let (_, e) = call(
        &app,
        Method::PUT,
        &uri,
        Some(json!({ "customFields": { "flag": "yes" } })),
    )
    .await;
    assert_eq!(e["fields"]["customFields.flag"], "invalid");
    // The 1b alias only touches the note.
    let (status, d) = call(
        &app,
        Method::PUT,
        &format!("/api/documents/{}/internal-note", id(&doc)),
        Some(json!({ "internalNote": "jen poznámka" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        (&d["internalNote"], &d["categoryId"]),
        (&json!("jen poznámka"), &json!(cat))
    );
    // Clearing.
    let (_, d) = call(&app, Method::PUT, &uri, Some(json!({}))).await;
    assert_eq!(
        (&d["categoryId"], &d["customFields"], &d["internalNote"]),
        (&Value::Null, &json!({}), &Value::Null)
    );
}
