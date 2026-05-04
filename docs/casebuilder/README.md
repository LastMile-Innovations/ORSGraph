# CaseBuilder Docs

These docs describe the live CaseBuilder product behavior and implementation contracts that are more durable than a backlog item.

## Guides

- [Markdown File Graph User Guide](markdown-file-graph-user-guide.md): User-facing guide for uploading Markdown files, opening the Markdown Graph panel, reviewing extracted entities/facts/timeline suggestions, using Markdown embedding search, and understanding `view_only` files.
- [Markdown File Graph Internal Reference](markdown-file-graph-internal.md): Engineering reference for the Markdown-only AST/indexing pipeline, Voyage embeddings, Neo4j graph contract, provenance rules, API response fields, UI wiring, and verification commands.
- [Upload Process and Storage](upload-process-storage.md): Diagrams for single and multipart direct uploads, bucket storage, upload sessions, resume/cancel behavior, and document processing states.

## Current Mode

CaseBuilder indexing is currently Markdown-only:

- Markdown files are editable, extractable, promotable into `WorkProduct.document_ast`, and indexable.
- Non-Markdown files are accepted and stored, but remain `view_only` for indexing. They can still be opened/downloaded and annotated where the workspace supports it.
- Voyage embeddings are Markdown-only and optional. They cover full files, chunks, and semantic units, with centroid fallback for oversized files.
- Extracted facts, timeline suggestions, and entity matches are review-first. They do not become operative matter records unless a user explicitly approves or links them.
