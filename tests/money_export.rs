//! Money S3 XML (`GET /api/export/accountant?format=money`): every exported
//! type in both directions validated against the vendored Money S3 XSD,
//! settings codes, `Doklad` omission, unexportable documents, empty period.

mod common;

use axum::http::{Method, StatusCode};
use common::csv_export::{export_error, get_raw, import_all};
use common::csvio::{file, row_in};
use common::documents::{create_bank, create_contact, create_issued, item as line, set_company};
use common::money::{JANUARY, all_items, export, item, items};
use common::pohoda::{HEADER, fixture};
use common::{TestDb, call, router};
use sea_orm::ConnectionTrait;
use serde_json::json;

fn has(item: &str, parts: &[&str]) {
    for p in parts {
        assert!(item.contains(p), "{p}\nin\n{item}");
    }
}

#[tokio::test]
async fn every_type_in_both_directions() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    set_company(&app, true).await;
    create_bank(&app, "CZK").await;
    import_all(&app, &fixture()).await;
    let contact = create_contact(&app, json!({ "name": "Nativní Odběratel", "ico": null })).await;
    create_issued(
        &app,
        json!({ "contactId": contact, "issueDate": "2026-01-20", "taxPointDate": "2026-01-20",
                "lines": [line("1", "5", "21")] }),
    )
    .await;

    let (status, headers, bytes) = get_raw(&app, JANUARY).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers["content-disposition"],
        "attachment; filename=\"money-2026-01-01-2026-01-31.xml\""
    );
    assert_eq!(headers["cache-control"], "no-store");
    assert!(bytes.starts_with(b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<MoneyData "));
    let xml = export(&app, JANUARY).await;
    has(
        &xml,
        &[
            "<MoneyData ICAgendy=\"44444443\" description=\"Export z Invoice 2026-01-01–2026-01-31\" ExpDate=\"",
            "<Ulice>Улица 1</Ulice>",
        ],
    );
    // 17 documents − the proforma = 16.
    assert_eq!(all_items(&xml).len(), 16, "{xml}");
    assert_eq!(items(&xml, "FaktVyd").len(), 6);
    assert_eq!(items(&xml, "FaktVyd_DPP").len(), 2);
    assert_eq!(items(&xml, "FaktPrij").len(), 6);
    assert_eq!(items(&xml, "FaktPrij_DPP").len(), 2);
    assert!(!xml.contains("<Doklad>ZF-1<"), "no proforma: {xml}");

    has(
        item(&xml, "FaktVyd", "Faktura FA-1"),
        &[
            "<Doklad>FA-1</Doklad><EvCisDokl>FA-1</EvCisDokl>",
            "<Vystaveno>2026-01-15</Vystaveno><DatUcPr>2026-01-15</DatUcPr>\
             <PlnenoDPH>2026-01-15</PlnenoDPH><Splatno>2026-01-29</Splatno>",
            "<VarSymbol>1</VarSymbol><Druh>N</Druh>",
            "<SazbaDPH1>12</SazbaDPH1><SazbaDPH2>21</SazbaDPH2>",
            "<SouhrnDPH><Zaklad22>1000.00</Zaklad22><DPH22>210.00</DPH22></SouhrnDPH>\
             <Celkem>1210.00</Celkem>",
            "<DodOdb><ObchNazev>Fiktivní Odběratel s.r.o.</ObchNazev><ObchAdresa>\
             <Ulice>Zkušební 12</Ulice><Misto>Brno</Misto><PSC>60200</PSC><KodStatu>CZ</KodStatu>\
             </ObchAdresa><ICO>12345679</ICO><DIC>CZ12345679</DIC>",
        ],
    );
    has(
        item(&xml, "FaktVyd", "Dobropis DB-1 k FA-1"),
        &[
            "<Druh>N</Druh><Dobropis>true</Dobropis>",
            "<Zaklad22>-100.00</Zaklad22><DPH22>-21.00</DPH22></SouhrnDPH><Celkem>-121.00</Celkem>",
        ],
    );
    let debit = item(&xml, "FaktVyd", "Vrubopis VB-1 k FA-1");
    has(debit, &["<DPH22>10.50</DPH22>", "<Celkem>60.50</Celkem>"]);
    assert!(!debit.contains("Dobropis"), "{debit}");
    has(
        item(&xml, "FaktVyd_DPP", "Daňový doklad k platbě DD-1 k ZF-1"),
        &["<Druh>D</Druh>", "<Zaklad22>500.00</Zaklad22>"],
    );
    has(
        item(
            &xml,
            "FaktVyd_DPP",
            "Opravný daňový doklad k platbě OD-1 k DD-1",
        ),
        &[
            "<Druh>D</Druh><Dobropis>true</Dobropis>",
            "<Zaklad22>-100.00</Zaklad22>",
        ],
    );
    // Rounding in Zaklad0: Money's total is our payable (112).
    let simplified = item(&xml, "FaktVyd", "Zjednodušený daňový doklad ZJ-1");
    has(
        simplified,
        &[
            "<ZjednD>true</ZjednD>",
            "<SouhrnDPH><Zaklad0>-0.40</Zaklad0><Zaklad5>100.00</Zaklad5><DPH5>12.40</DPH5>\
             </SouhrnDPH><Celkem>112.00</Celkem>",
        ],
    );
    assert!(!simplified.contains("DodOdb"), "contactless");
    has(
        item(&xml, "FaktVyd", "Faktura FA-3"),
        &[
            "<Misto>München</Misto>",
            "<KodStatu>DE</KodStatu>",
            "<SouhrnDPH><Zaklad22>2500.00</Zaklad22><DPH22>525.00</DPH22></SouhrnDPH>\
             <Celkem>3025.00</Celkem><Valuty><Mena><Kod>EUR</Kod><Mnozstvi>1</Mnozstvi>\
             <Kurs>25</Kurs></Mena><SouhrnDPH><Zaklad22>100.00</Zaklad22><DPH22>21.00</DPH22>\
             </SouhrnDPH><Celkem>121.00</Celkem></Valuty>",
        ],
    );
    let native = items(&xml, "FaktVyd")
        .into_iter()
        .find(|i| i.contains("Nativní Odběratel"))
        .expect("native invoice");
    has(
        native,
        &["<Uhrada>převodem</Uhrada>", "<Celkem>6.05</Celkem>"],
    );

    let fv42 = item(&xml, "FaktPrij", "Přijatá faktura FV-42");
    has(
        fv42,
        &[
            "<DatUcPr>2026-01-22</DatUcPr>",
            "<Doruceno>2026-01-22</Doruceno>",
            "<PrijatDokl>FV-42</PrijatDokl>",
            "<SouhrnDPH><Zaklad22>2433.50</Zaklad22><DPH22>511.04</DPH22></SouhrnDPH>",
            "<Kurs>24.335</Kurs>",
            "<ObchNazev>Vzorový Dodavatel a.s.</ObchNazev>",
            "<ICO>87654326</ICO>",
        ],
    );
    assert!(
        !fv42.contains("KodDPH") && !fv42.contains("EvCisDokl"),
        "{fv42}"
    );
    has(
        item(&xml, "FaktPrij", "Přijatý dobropis FV-45 k FV-44"),
        &["<Dobropis>true</Dobropis>", "<Zaklad22>-200.00<"],
    );
    has(
        item(&xml, "FaktPrij", "Přijatý vrubopis FV-46 k FV-44"),
        &["<PrijatDokl>FV-46</PrijatDokl>"],
    );
    has(
        item(&xml, "FaktPrij_DPP", "Přijatý daňový doklad k platbě FV-47"),
        &["<Druh>D</Druh>", "<Zaklad22>1000.00<"],
    );
    has(
        item(
            &xml,
            "FaktPrij_DPP",
            "Přijatý opravný doklad k platbě FV-48 k FV-47",
        ),
        &["<Dobropis>true</Dobropis>", "<Zaklad22>-100.00<"],
    );
    has(
        item(&xml, "FaktPrij", "Přijatý zjednodušený daňový doklad FV-49"),
        &["<SouhrnDPH><Zaklad0>100.00</Zaklad0></SouhrnDPH>"],
    );
    // 10 % has no standard slot: Money's further rates (with Zaklad0, so Money
    // never reads the standard rates from the list), no third-rate setting needed.
    has(
        item(&xml, "FaktPrij", "Přijatá faktura FV-50"),
        &[
            "<SouhrnDPH><Zaklad0>0.00</Zaklad0><SeznamDalsiSazby><DalsiSazba>\
             <HladinaDPH>1</HladinaDPH><Sazba>10</Sazba><Zaklad>100.00</Zaklad><DPH>10.00</DPH>\
             </DalsiSazba></SeznamDalsiSazby></SouhrnDPH><Celkem>110.00</Celkem>",
        ],
    );
    for absent in ["PredKontac", "KodDPH", "<Rada>"] {
        assert!(!xml.contains(absent), "no codes set: {absent}");
    }

    // Direction filter.
    let issued = export(&app, &format!("{JANUARY}&direction=issued")).await;
    assert_eq!(all_items(&issued).len(), 8);
    assert!(!issued.contains("SeznamFaktPrij"));
}

#[tokio::test]
async fn settings_codes_doklad_and_unexportable() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    set_company(&app, true).await;
    import_all(&app, &fixture()).await;
    let long = row_in(
        HEADER,
        &[
            ("direction", "issued"),
            ("doc_type", "invoice"),
            ("number", "FA-2026-000123"),
            ("counterparty_name", "Fiktivní Odběratel s.r.o."),
            ("counterparty_ico", "12345679"),
            ("issue_date", "28.01.2026"),
            ("base_21", "100"),
            ("vat_21", "21"),
            ("total", "121"),
        ],
    );
    import_all(&app, &file(HEADER, &[&long])).await;
    let (status, body) = call(
        &app,
        Method::PUT,
        "/api/settings/accounting",
        Some(json!({ "money": { "ico": "12345679", "codes": [
            { "direction": "issued", "docType": "invoice", "accounting": "3Fv",
              "classificationVat": "UD", "numberSeries": "FV" },
            { "direction": "received", "docType": "advance_tax_doc", "accounting": "1Zal" },
            { "direction": "received", "docType": "invoice", "classificationVat": "PD",
              "classificationVatNonDeductible": "PN" },
        ]}})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let xml = export(&app, JANUARY).await;
    assert!(
        xml.contains("<MoneyData ICAgendy=\"12345679\" "),
        "override"
    );
    has(
        item(&xml, "FaktVyd", "Faktura FA-1"),
        &[
            "<Doklad>FA-1</Doklad><EvCisDokl>FA-1</EvCisDokl><Rada>FV</Rada>",
            "<KodDPH>UD</KodDPH>",
            "<PredKontac>3Fv</PredKontac>",
        ],
    );
    // Longer than Money's 10: no Doklad, Money numbers it in the FV series.
    let long = item(&xml, "FaktVyd", "Faktura FA-2026-000123");
    assert!(
        long.starts_with("<EvCisDokl>FA-2026-000123</EvCisDokl><Rada>FV</Rada>"),
        "{long}"
    );
    let credit = item(&xml, "FaktVyd", "Dobropis DB-1 k FA-1");
    assert!(
        !credit.contains("Rada") && !credit.contains("PredKontac"),
        "{credit}"
    );
    has(
        item(&xml, "FaktPrij_DPP", "Přijatý daňový doklad k platbě FV-47"),
        &["<PredKontac>1Zal</PredKontac>"],
    );
    has(
        item(&xml, "FaktPrij", "Přijatá faktura FV-42"),
        &["<KodDPH>PN</KodDPH>"],
    );
    has(
        item(&xml, "FaktPrij", "Přijatá faktura FV-44"),
        &["<KodDPH>PD</KodDPH>"],
    );

    // A variable symbol Money cannot hold (legacy data): no file, the
    // accountant is told which document and why.
    db.conn
        .execute_unprepared(
            "UPDATE documents SET variable_symbol = '123456789012345678901' WHERE number = 'FA-1'",
        )
        .await
        .expect("update");
    let (status, body) = export_error(&app, JANUARY).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        body,
        json!({ "code": "validation", "fields": { "documents": "unexportable" },
                "detail": "FA-1: variable symbol longer than 20: 123456789012345678901" })
    );
}

#[tokio::test]
async fn an_empty_period() {
    let db = TestDb::new().await;
    let app = router(db.conn.clone());
    set_company(&app, true).await;
    let (status, body) = export_error(&app, JANUARY).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        body,
        json!({ "code": "validation", "fields": { "from": "empty" } })
    );
}
