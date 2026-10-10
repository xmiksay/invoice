//! Money S3 XML export: requests and XSD validation against the vendored
//! Money S3 schemas (`_Document.xsd`). The documents come from
//! [`super::pohoda::fixture`].

use axum::Router;
use axum::http::StatusCode;

use super::csv_export::get_raw;

pub const JANUARY: &str = "/api/export/accountant?from=2026-01-01&to=2026-01-31&format=money";

/// GET a Money export; asserts 200, the headers and XSD validity, and
/// returns the text.
pub async fn export(app: &Router, uri: &str) -> String {
    let (status, headers, bytes) = get_raw(app, uri).await;
    let text = String::from_utf8(bytes.clone()).expect("UTF-8");
    assert_eq!(status, StatusCode::OK, "{text}");
    assert_eq!(headers["content-type"], "application/xml; charset=utf-8");
    super::assert_xsd(&bytes, "money/schema/_Document.xsd", &text);
    text
}

/// The items of the list element `list` (e.g. `FaktVyd`), in order.
pub fn items<'a>(xml: &'a str, list: &str) -> Vec<&'a str> {
    let open = format!("<{list}>");
    let close = format!("</{list}>");
    xml.split(&open)
        .skip(1)
        .filter_map(|i| i.split(&close).next())
        .collect()
}

/// Every item of every list, in file order.
pub fn all_items(xml: &str) -> Vec<&str> {
    ["FaktPrij", "FaktVyd", "FaktPrij_DPP", "FaktVyd_DPP"]
        .into_iter()
        .flat_map(|l| items(xml, l))
        .collect()
}

/// The item of `list` whose `Popis` is `popis`.
pub fn item<'a>(xml: &'a str, list: &str, popis: &str) -> &'a str {
    let needle = format!("<Popis>{popis}</Popis>");
    items(xml, list)
        .into_iter()
        .find(|i| i.contains(&needle))
        .unwrap_or_else(|| panic!("no {list} {popis} in\n{xml}"))
}
