# Money S3 XML schemas

Vendored unchanged (UTF-8) for the Money S3 export tests: every exported file is validated with
`xmllint --schema _Document.xsd` (`tests/common/money.rs`, `tests/common/mod.rs` `assert_xsd`).

- These are the files Money S3 ships in its installation folder `Data/XMLDE/Schemas`.
- Source: copied from the public `WeblateOrg/website` repository, directory `schemas/money-s3/`
  (<https://github.com/WeblateOrg/website/tree/main/schemas/money-s3>), commit dated 2024-10-22.
- Copied: 2026-10-10.
- Files: `_Document.xsd` (root `MoneyData`) and its whole `xs:include` closure — `__Comtypes.xsd`, `__Faktura.xsd`,
  `__Firma.xsd`, `__IntDokl.xsd`, `__InvDokl.xsd`, `__Mzda.xsd`, `__Objedn.xsd`, `__Seznamy.xsd`, `__SklDokl.xsd`,
  `__UcDokl.xsd`, `__Uhrady.xsd`, `__Zakazka.xsd`, `__Zamestnanec.xsd`, `__Zasoba.xsd` and `_Report.xsd` (included
  by `__Faktura.xsd` / `__SklDokl.xsd`), 15 files. `_Export.xsd` / `_Import.xsd` (the export / import definition
  files) are not needed and not vendored.

To refresh: copy the same file names from a Money S3 installation (or the repository above), follow every
`xs:include` until no file is missing, and re-run `make test-integration`.
