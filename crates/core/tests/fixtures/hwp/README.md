# HWP import fixtures

`hello_world.hwp` and `draw_text.hwp` are Hancom HWP 5.1.0.1 test documents
from [hahnlee/hwp-rs](https://github.com/hahnlee/hwp-rs), as packaged in the
`hwp` 0.2.0 crate under `tests/integration/project/files/`. The upstream
Apache-2.0 license is included as `LICENSE`. These files exercise genuine
Hancom CFB records and compression, including the retained CRC32/ISIZE footer.
`draw_text.hwp` contains shape text that must be omitted from manuscript import.

SHA-256:

- `hello_world.hwp`: `283e16ee51834d75e22f413203549027a5d3fda39346302d9429e9732bb3a77c`
- `draw_text.hwp`: `40b34944c664914bb4948778683c7e2e49f067b0ef3a8d27287a58124877e092`

Additional synthetic fixtures are built in `import_hwp.rs` to test Unicode,
multiple sections, formatting, damaged streams and unsupported document modes.
