# Modification notice

This skill is adapted from
[`alirezarezvani/claude-code-aso-skill`](https://github.com/alirezarezvani/claude-code-aso-skill),
revision `94148561f173a917b45f8fd125e3025fa25cba85`.

Copyright (c) 2025 Alireza Rezvani. Used under MIT license in
[`LICENSE.txt`](LICENSE.txt).

Changes:

- rewrote Claude-specific workflow for Codex skills and progressive disclosure;
- replaced Python package with focused Rust metadata/experiment validator;
- corrected Google Play app-name limit from 50 to current 30 characters;
- enforced Apple's keyword limit as 100 UTF-8 bytes rather than characters;
- removed sample metrics and unsupported search-volume/ranking claims;
- composed repository SEO and autoresearch skills instead of duplicating agents;
- added current authoritative platform sources and boundary tests.
