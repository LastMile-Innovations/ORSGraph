# Markdown File Graph User Guide

The Markdown File Graph helps you see what CaseBuilder understands from your Markdown case files. It turns the document structure, source spans, extracted entities, proposed facts, and timeline suggestions into a reviewable graph.

## What It Does

For Markdown files, CaseBuilder can:

- Keep the original source text editable in the Document Workspace.
- Build an outline from headings, quotes, lists, tables, code blocks, links, images, and text.
- Extract reviewable entities such as parties, organizations, courts, places, statutes, dates, rules, and money amounts.
- Suggest facts and timeline items from exact source spans.
- Connect graph items back to the source location in the Markdown file.
- Build Voyage embeddings for Markdown files, chunks, and semantic units when embeddings are enabled.
- Search the embedded Markdown graph and jump back to exact source ranges.
- Show the document in the matter graph with Markdown, AST, entity, and provenance views.

CaseBuilder does not automatically treat extracted material as final. Facts, timeline items, and entity matches remain review-first.

## Markdown-Only Indexing

The active CaseBuilder indexing mode is Markdown-only.

Markdown files can be:

- Edited.
- Extracted.
- Indexed.
- Promoted into a WorkProduct.
- Used to generate source-backed facts, entity mentions, and timeline suggestions.

Non-Markdown files can still be:

- Uploaded.
- Stored privately.
- Opened or downloaded.
- Previewed when supported.
- Annotated where the workspace supports annotation.

Non-Markdown files are marked `view_only` for indexing. They are not extracted or promoted into the CaseBuilder AST in the current mode.

## Uploading Files

You can upload individual files or folders. Folder paths are preserved as private library metadata so you can keep source organization without leaking path names into generated storage keys.

After upload:

- Markdown files are eligible for indexing.
- Non-Markdown files stay in the document library as stored sources.
- Mixed uploads are okay. CaseBuilder indexes the Markdown files and skips the rest with a visible reason.

## Using The Markdown Graph Panel

Open a Markdown document from the document library, then choose the Markdown Graph tab in the Document Workspace.

The panel shows:

- `AST nodes`: the parsed Markdown structure.
- `Entities`: reviewable people, parties, organizations, courts, places, statutes, dates, rules, and money mentions.
- `Chunks`: indexed text units.
- `Spans`: exact source spans used for provenance.

The Outline section lists high-level Markdown structure. Clicking an outline item jumps to the corresponding source range in the Markdown editor when offsets are available.

The Entities section shows canonical review candidates. Party-like entities may show party candidates, but CaseBuilder does not mutate the party list automatically.

The Facts And Timeline section shows proposed facts and suggested timeline items tied back to source spans and AST nodes.

## Markdown Embeddings And Search

When CaseBuilder Markdown embeddings are enabled, the Markdown Graph panel also shows embedding coverage.

Coverage includes:

- Current embedded records.
- Embedded chunks.
- Embedded semantic units.
- Stale records from older document versions.
- Latest embedding run status.

The Embed action reruns embeddings for the current Markdown document. This is safe to retry. Provider failures do not undo extraction or remove reviewable graph data.

The Semantic Search box searches the embedded Markdown graph for the current document. Results show the matched target, score, excerpt, and stale status. Clicking a result jumps to the source range when AST offsets are available.

CaseBuilder embeds:

- The full Markdown file when it is small enough to send without truncation.
- Text chunks.
- Markdown semantic units.

For large Markdown files, CaseBuilder does not silently truncate legal source text. It builds a full-file centroid from chunk and semantic-unit vectors instead.

## Matter Graph Modes

The Matter Graph viewer includes Markdown-aware modes when the backend provides them:

- `markdown`: document, version, AST, chunk, and source-span graph.
- `markdown_ast`: parsed Markdown document and AST tree.
- `markdown_embeddings`: embedding runs, records, chunks, semantic units, and provenance edges.
- `entities`: entity mentions, canonical entities, and party candidate links.
- `provenance`: source spans, chunks, evidence spans, search records, index runs, manifests, facts, events, and AST links.

These modes are compact audit views. They intentionally avoid putting full document text into the graph display.

## Review Rules

Use the Markdown Graph as a review tool:

- Approve facts only after checking their source.
- Approve timeline suggestions only when the date and event wording match the source.
- Treat entity matches as candidates until reviewed.
- Link party-like entities to existing parties explicitly.
- Leave uncertain, incomplete, or unsupported items unapproved.

AI/provider enrichment, when enabled, may improve labels, grouping, confidence, explanations, or warnings. It may not invent facts, events, source spans, or AST nodes.

## Common States

| State | Meaning |
| --- | --- |
| `processed` | The Markdown file was extracted and indexed. |
| `review_ready` | CaseBuilder produced reviewable extracted material. |
| `view_only` | The file is stored but not indexable in the active Markdown-only mode. |
| `unsupported` | CaseBuilder cannot process this file type in the active indexing mode. |
| `ocr_required` | Future parser/OCR work is needed before this source can be indexed. |

## Practical Workflow

1. Upload a folder or set of files.
2. Let CaseBuilder index the Markdown files.
3. Open a Markdown document.
4. Review the Markdown Graph panel.
5. Click outline, fact, or timeline items to inspect source ranges.
6. Approve or promote only the items that match the source.
7. Open the Matter Graph and switch to Markdown or provenance modes to inspect the broader case graph.

The goal is to make source review faster, not to bypass review.
