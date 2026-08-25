# Documentation instructions

This directory owns maintained architecture prose and documentation assets.
Root `README.md`, `ROADMAP.md`, and `CHANGELOG.md` remain canonical; the docs
build copies them into its generated source tree.

Keep capability statements aligned with executable behavior. Never copy local
standards references into docs or generated site inputs. Run
`./scripts/build-docs.sh` after documentation changes.
