# DA-001-reference-checker-name: Name for the spec-reference checker tool

**Status:** Accepted
**Date:** 2026-05-08

## 1. Context

We need a name for the tool that verifies the ID-based reference scheme defined in `agents.md` (e.g. `FS-042-user-login.3.1`). Working title is `spec-checker`. We want something memorable, available on every registry we plan to publish to, and free of confusing brand collisions.

The tool is written in Rust, but it will be distributed on **all three** registries — cargo, npm, and PyPI — with native API bindings on each (Node via napi-rs or similar; Python via PyO3/maturin). Cargo, npm, and PyPI availability are therefore **all required**.

## 2. Shortlist

Availability checked on 2026-05-08. "Really used?" answers whether the existing taker on a given registry is an active project worth worrying about (active collision) or dormant/niche (namespace squat we can ignore).

| Name           | cargo                          | npm                              | PyPI                             | Really used?                                                                                       | Vibe / metaphor                                                            |
|----------------|--------------------------------|----------------------------------|----------------------------------|-----------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------|
| **fiducial**   | FREE                           | FREE                             | FREE                             | n/a — all free                                                                                      | Reference mark establishing a coordinate system. Sharpest metaphor of all. |
| **gnd**        | FREE                           | TAKEN — "Ground Web Framework"   | TAKEN — "GraphAnomalyDection"    | npm: dormant (~860/mo, last release 2022). pypi: niche academic, 7 releases 2022. Both ignorable.   | Engineer-terse abbreviation for "ground." Hacker feel.                      |
| **endnote**    | FREE                           | FREE                             | FREE                             | n/a — all free; but "EndNote" is Clarivate's commercial citation manager (different domain).        | A numbered reference at end of text. Direct meaning, brand collision.       |
| **vademecum**  | FREE                           | FREE                             | FREE                             | n/a — all free                                                                                      | Latin "go with me" — a handbook. Authentic but obscure and 9 letters.       |
| **dogtag**     | FREE                           | FREE                             | FREE                             | n/a — all free                                                                                      | Military ID tag. Punchy, no metaphor stretch.                                |
| **marginalia** | FREE                           | TAKEN — "code-block runner"      | TAKEN — "Obsidian vault scanner" | npm: 38/mo, last 2022, dead. pypi: 2 releases, 2026, tiny but live. Mild SEO collision.             | Notes a reader writes in margins pointing elsewhere. Best literary metaphor. |
| **lodestar**   | FREE                           | TAKEN — "MVC for JavaScript"     | TAKEN — alpha 2018               | npm: 14/mo, dead. pypi: single 2018 release. Both ignorable.                                        | Guiding star. References guide through the spec.                            |
| **palimpsest** | FREE                           | TAKEN — "picture merger"         | TAKEN — empty desc               | npm: 38/mo, dead. pypi: 2 releases, dead. Both ignorable.                                           | Manuscript reused after erasing — fits superseded decisions.                |
| **codicil**    | FREE                           | TAKEN — empty desc               | FREE                             | npm: 70/mo, fresh (2026-04) but no description. Probably parked or anonymous. Mild risk.            | Legal addendum to a will — fits decision records.                            |
| **specref**    | FREE                           | FREE                             | FREE                             | n/a — all free                                                                                      | Plain descriptive. Forgettable.                                              |
| **idlint**     | FREE                           | FREE                             | FREE                             | n/a — all free                                                                                      | Plain descriptive.                                                           |
| **specdex**    | FREE                           | FREE                             | FREE                             | n/a — all free                                                                                      | Plain descriptive.                                                           |
| **citecheck**  | FREE                           | FREE                             | TAKEN — "citation-chain protection" | pypi: 5 releases 2023, dead.                                                                     | Plain descriptive.                                                           |
| **refcheck**   | FREE                           | FREE                             | TAKEN — "broken refs in Markdown" | pypi: 12 releases, last 2026-04, actively maintained. **Functional overlap: zero** — it checks standard markdown link syntax (`[x](url)`, anchors, file paths), basically a Python `lychee`. **Naming collision: real** — same domain, generic name, SEO confusion. | Plain descriptive — and brand-conflicted.                                   |
| **based**      | TAKEN — "Custom numeral systems" (11.5k dl) | TAKEN — "number base utility" | FREE                  | cargo: real, used package. **Hard collision.** npm: dead.                                          | Internet-meme term. Dated, unsearchable, and cargo is taken.                |

## 3. Options

### 3.1 `fiducial` (recommended)

Free on all three registries, no brand collision, technical register fits Rust ecosystem, sharpest metaphor: a fiducial is a reference mark establishing a coordinate system — exactly what the IDs are.

**Cost:** 9 letters; mildly esoteric (most users will look it up once).

### 3.2 `gnd`

Free on cargo. Taken but dormant on npm and PyPI. Pair with `gnd-cli` on npm if we ever publish there (`gnd-cli`, `gnd-check`, `@gnd/cli` are all free).

**Cost:** poor SEO ("gnd" is the universal EE abbreviation for ground); existing dormant packages share the name.

### 3.3 `endnote`

Free everywhere. Familiar word, direct meaning.

**Cost:** brand collision with EndNote (Clarivate citation manager) — different domain but searchability suffers.

### 3.4 Bland descriptive (`specref`, `idlint`, `specdex`, `dogtag`)

Free everywhere. Zero collision risk.

**Cost:** unmemorable, no character.

## 4. Decision

**`gnd`.** Three-letter binary, hacker-terse, cargo-clean. The npm and PyPI namespace squats are dormant and ignorable. SEO is mediocre but acceptable; "gnd rust" or "gnd spec" disambiguates fine.

When publishing to npm in future, the package will be named `gnd-cli` (also free across all three registries).

## 5. Consequences

- The chosen name applies to the cargo crate, the binary, and (eventually) the GitHub repo.
- If we pick `gnd`, the npm package — when published — will be `gnd-cli` (also free on cargo and PyPI for consistency).
- `refcheck` is **eliminated** from consideration: not because the existing PyPI tool does the same thing (it doesn't — it's a markdown link checker, scope-disjoint from us), but because it occupies the same naming territory and would create persistent SEO and mental-model confusion.
- This decision affects only the tool. The reference scheme itself is decided in `agents.md` and is independent.
