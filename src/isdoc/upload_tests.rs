use std::io::Write as _;

use super::*;

/// A zip of `(name, bytes)` entries (deflated).
pub fn zip_of(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (name, bytes) in entries {
        w.start_file(*name, opts).expect("start entry");
        w.write_all(bytes).expect("write entry");
    }
    w.finish().expect("finish zip").into_inner()
}

fn file(name: &str, bytes: Vec<u8>) -> File {
    File {
        name: name.into(),
        bytes,
    }
}

fn keys(found: &[Found]) -> Vec<&str> {
    found.iter().map(|f| f.key.as_str()).collect()
}

#[test]
fn zip_in_zip_is_unpacked_in_order() {
    let inner = zip_of(&[("b.isdoc", b"<b/>"), ("notes.txt", b"x")]);
    let outer = zip_of(&[
        ("a.isdoc", b"<a/>"),
        ("2026.zip", &inner),
        ("dir/", b""),
        ("readme.md", b"ignored"),
    ]);
    let found = unpack(
        vec![
            file("invoices.zip", outer),
            file("c.isdoc", b"<c/>".to_vec()),
        ],
        LIMITS,
    )
    .expect("unpacked");
    assert_eq!(
        keys(&found),
        [
            "invoices.zip/a.isdoc",
            "invoices.zip/2026.zip/b.isdoc",
            "c.isdoc"
        ]
    );
    assert_eq!(found[1].doc.as_ref().expect("doc").xml, b"<b/>");
}

#[test]
fn same_name_gets_a_suffix() {
    let found = unpack(
        vec![
            file("a.isdoc", b"<a/>".to_vec()),
            file("a.isdoc", b"<a/>".to_vec()),
            file("a.isdoc", b"<a/>".to_vec()),
        ],
        LIMITS,
    )
    .expect("unpacked");
    assert_eq!(keys(&found), ["a.isdoc", "a.isdoc#2", "a.isdoc#3"]);
}

#[test]
fn depth_limit() {
    let l3 = zip_of(&[("x.isdoc", b"<x/>")]);
    let l2 = zip_of(&[("l3.zip", &l3)]);
    let l1 = zip_of(&[("l2.zip", &l2)]);
    let ok = unpack(vec![file("l1.zip", l1.clone())], LIMITS).expect("unpacked");
    assert_eq!(keys(&ok), ["l1.zip/l2.zip/l3.zip/x.isdoc"]);
    let l0 = zip_of(&[("l1.zip", &l1)]);
    let deep = unpack(vec![file("l0.zip", l0)], LIMITS).expect("unpacked");
    assert_eq!(keys(&deep), ["l0.zip/l1.zip/l2.zip/l3.zip"]);
    assert_eq!(deep[0].doc, Err(TOO_DEEP));
}

#[test]
fn limits_count_real_bytes_and_documents() {
    // 1 MiB of zeros compresses to ~1 KiB: only the decompressed size counts.
    let big = vec![0u8; 1024 * 1024];
    let bomb = zip_of(&[("a.isdoc", &big), ("b.isdoc", &big)]);
    assert!(bomb.len() < 64 * 1024);
    let tight = Limits {
        max_unpacked: 1024 * 1024 + 10,
        ..LIMITS
    };
    assert_eq!(
        unpack(vec![file("bomb.zip", bomb.clone())], tight),
        Err(Exceeded::TooLarge)
    );
    assert!(unpack(vec![file("bomb.zip", bomb)], LIMITS).is_ok());
    let few = Limits {
        max_docs: 2,
        ..LIMITS
    };
    let three = zip_of(&[("1.isdoc", b"1"), ("2.isdoc", b"2"), ("3.isdoc", b"3")]);
    assert_eq!(
        unpack(vec![file("x.zip", three)], few),
        Err(Exceeded::TooMany)
    );
}

#[test]
fn broken_archives_are_entry_errors() {
    let found = unpack(
        vec![
            file("bad.zip", b"PK nope".to_vec()),
            file("bad.isdocx", b"nope".to_vec()),
        ],
        LIMITS,
    )
    .expect("unpacked");
    assert_eq!(found[0].doc, Err(INVALID_ARCHIVE));
    assert_eq!(found[1].doc, Err(INVALID_XML));
}

#[test]
fn isdocx_main_document_and_pdfs() {
    let manifest = br#"<?xml version="1.0"?><manifest xmlns="http://isdoc.cz/namespace/2013/manifest"><maindocument filename="real.isdoc"/></manifest>"#;
    let x = zip_of(&[
        ("other.isdoc", b"<other/>"),
        ("manifest.xml", manifest),
        ("real.isdoc", b"<real/>"),
        ("faktura.pdf", b"%PDF-1.4 x"),
    ]);
    let found = unpack(vec![file("f.isdocx", x)], LIMITS).expect("unpacked");
    let doc = found[0].doc.as_ref().expect("doc");
    assert_eq!(doc.xml, b"<real/>");
    assert_eq!(
        doc.pdf.as_ref().map(|p| p.name.as_str()),
        Some("faktura.pdf")
    );
    // Without a manifest: the root `.isdoc`.
    let y = zip_of(&[("sub/a.isdoc", b"<sub/>"), ("root.isdoc", b"<root/>")]);
    let found = unpack(vec![file("g.isdocx", y)], LIMITS).expect("unpacked");
    assert_eq!(found[0].doc.as_ref().expect("doc").xml, b"<root/>");
    let none = zip_of(&[("a.pdf", b"%PDF-")]);
    let found = unpack(vec![file("h.isdocx", none)], LIMITS).expect("unpacked");
    assert_eq!(found[0].doc, Err(INVALID_XML));
}

#[test]
fn oversized_pdf_is_skipped() {
    let small = Limits {
        max_pdf: 4,
        ..LIMITS
    };
    let x = zip_of(&[("a.isdoc", b"<a/>"), ("a.pdf", b"%PDF-1.4 too big")]);
    let found = unpack(vec![file("a.isdocx", x)], small).expect("unpacked");
    let doc = found[0].doc.as_ref().expect("doc");
    assert_eq!(doc.pdf.as_ref().and_then(|p| p.bytes.as_ref()), None);
    assert_eq!(pdf_bytes(doc.pdf.as_ref()), Err(PDF_SKIPPED));
}

#[test]
fn pdf_choice() {
    let names = |n: &[&str]| n.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    assert_eq!(choose_pdf(&names(&["a.isdoc", "x.pdf"]), None), Some(1));
    let two = names(&["a.pdf", "b.pdf", "a.isdoc"]);
    assert_eq!(choose_pdf(&two, None), None);
    assert_eq!(choose_pdf(&two, Some("b.pdf")), Some(1));
    assert_eq!(choose_pdf(&two, Some("missing.pdf")), None);
    let pdf = |bytes: &[u8]| Pdf {
        name: "x.pdf".into(),
        bytes: Some(bytes.to_vec()),
    };
    assert_eq!(
        pdf_bytes(Some(&pdf(b"%PDF-1"))),
        Ok(Some(bytes::Bytes::from_static(b"%PDF-1")))
    );
    assert_eq!(pdf_bytes(Some(&pdf(b"<html>"))), Ok(None));
    assert_eq!(pdf_bytes(None), Ok(None));
}

#[test]
fn only_the_chosen_pdf_is_decompressed() {
    // Two 1 MiB PDFs, no preview: neither is chosen, so neither is read and
    // the tight unpacked budget holds.
    let big = [b"%PDF-".as_slice(), &vec![0u8; 1024 * 1024]].concat();
    let x = zip_of(&[("a.isdoc", b"<a/>"), ("a.pdf", &big), ("b.pdf", &big)]);
    let tight = Limits {
        max_unpacked: 64 * 1024,
        ..LIMITS
    };
    let found = unpack(vec![file("a.isdocx", x)], tight).expect("unpacked");
    assert_eq!(found[0].doc.as_ref().expect("doc").pdf, None);
    // The preview supplement names one: only that one counts.
    let xml = br#"<Invoice xmlns="http://isdoc.cz/namespace/2013"><SupplementsList><Supplement preview="true"><Filename>b.pdf</Filename></Supplement></SupplementsList></Invoice>"#;
    let y = zip_of(&[("a.isdoc", xml), ("a.pdf", &big), ("b.pdf", &big)]);
    let roomy = Limits {
        max_unpacked: 1024 * 1024 + 4096,
        ..LIMITS
    };
    let found = unpack(vec![file("b.isdocx", y)], roomy).expect("unpacked");
    let pdf = found[0]
        .doc
        .as_ref()
        .expect("doc")
        .pdf
        .clone()
        .expect("pdf");
    assert_eq!(pdf.name, "b.pdf");
    assert_eq!(pdf.bytes.map(|b| b.len()), Some(big.len()));
}
