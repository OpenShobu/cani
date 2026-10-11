# Upstream conformance snapshot

Files in this directory are copied unchanged from
https://github.com/DaShobu/Universal-Shobu-Protocol-Specifications (Apache-2.0).
Retrieved 2026-10-11. They are retained together so the conformance runner's
relative imports work. This is test tooling, not a runtime dependency.

Run from cani's root with the absolute path to the built USP executable:

```text
python tests/upstream/tests/conformance.py --engine /absolute/path/to/cani-usp
```
