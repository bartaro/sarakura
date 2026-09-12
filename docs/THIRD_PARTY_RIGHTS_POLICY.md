# Third-party rights policy

SARAKURA source, documentation, catalogs, tests, fixtures, standard output
templates, and reports use generic hardware terminology and synthetic data.
They must not contain a third-party work title, product-specific proper noun,
character, location, item, ROM hash, ROM byte sequence, image, audio, or
extracted table.

Project paths, labels, identifiers, and input hashes are redacted by default.
Reproduction bundles omit raw input files. Private labels may be preserved only
with an explicit local opt-in and must not be committed to shared artifacts.

`tools/rights_name_guard.py` accepts a private external denylist with one term
per line. Keep that list in `deny_terms.local.txt`; it is ignored by version
control. The guard reports only file and line locations so protected terms are
not repeated into standard logs.
