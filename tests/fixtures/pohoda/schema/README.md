# Stormware POHODA XML schemas, version 2

Vendored unchanged (Windows-1250, CRLF) for the Pohoda export tests: every exported file is validated with
`xmllint --schema data.xsd` (`tests/common/pohoda.rs`).

- Source: `https://www.stormware.cz/xml/schema/version_2/<file>.xsd` — `data.xsd`, `invoice.xsd`, `intDoc.xsd`,
  `type.xsd` and every schema they import (transitively; `data.xsd` imports all agendas), 73 files.
- Downloaded: 2026-10-10.
- Documentation: <https://www.stormware.cz/xml/> (schema annotations are in Czech).

To refresh: download the same file names again (follow `schemaLocation` of every `xsd:import` until no file is
missing) and re-run `make test-integration`.
