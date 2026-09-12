# SUROM 512 KiB diagnostics

SARAKURA's FC catalog contains generic rules for MMC1 outer-bank state,
common-bank replication, serial-write interruption and adjacent-cycle ignore,
unsafe CHR mode, PRG-RAM disable, logical bank range, cartridge header, vector
replication, and build-metadata mapping.

These rules use board and hardware terminology only. They do not identify or
embed any commercial work, asset, ROM hash, ROM byte sequence, image, audio,
or extracted table.

Project labels are redacted by default during analysis. Build IDs, input paths,
source files, function and operation labels, snapshot references, and trace
references become deterministic generic identifiers; the input ROM hash is
omitted. Use `--allow-project-labels` only for an explicitly authorized local
workflow that needs original source correlation.

Reproduction bundles contain derived diagnostics, plans, and reports. Raw
metadata and event input files are omitted even when label preservation is
enabled. The bundle manifest records `source_inputs_omitted: true`.

The fixture in `tests/fixtures/surom512` is synthetic and contains no ROM data.
It proves catalog matching, evidence generation, repair targeting, confidence,
retest conditions, and default redaction.
