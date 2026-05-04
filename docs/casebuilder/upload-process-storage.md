# CaseBuilder Upload Process and Storage

This diagram documents the live CaseBuilder direct-upload flow. The API creates private matter/document records and signed bucket URLs; the browser sends bytes directly to the object store.

## End-to-End Flow

```mermaid
flowchart TD
  User["User selects files or folders"] --> UI["CaseBuilder upload provider"]
  UI --> Intent["POST /matters/{matter_id}/files/uploads"]
  Intent --> Validate["API validates auth, matter access, metadata, size limits"]
  Validate --> CreateDoc["Create pending CaseDocument"]
  CreateDoc --> Choose{"bytes > single upload threshold?"}

  Choose -- "No" --> SingleIntent["Presign one PUT URL"]
  SingleIntent --> SinglePut["Browser PUTs file to bucket"]
  SinglePut --> SingleComplete["POST /uploads/{upload_id}/complete"]

  Choose -- "Yes" --> MultipartStart["Create S3 multipart upload"]
  MultipartStart --> Session["Persist private UploadSession"]
  Session --> PartIntents["Return initial signed part URLs"]
  PartIntents --> PartPuts["Browser uploads file.slice(...) parts"]
  PartPuts --> PartList["GET /uploads/{upload_id}/parts for resume"]
  PartList --> MultipartComplete["POST /uploads/{upload_id}/complete with ordered part ETags"]

  SingleComplete --> StoreDone["Mark document stored"]
  MultipartComplete --> StoreDone
  StoreDone --> TypeGate{"Markdown file?"}
  TypeGate -- "Yes" --> Indexable["processing_status = uploaded / indexable"]
  TypeGate -- "No" --> ViewOnly["processing_status = view_only"]
  Indexable --> OptionalIndex["Optional indexing extracts facts, spans, AST, timeline suggestions"]
  ViewOnly --> Library["Stored source is openable/downloadable in document library"]
  OptionalIndex --> Library
```

## Multipart Upload Sequence

```mermaid
sequenceDiagram
  autonumber
  participant B as Browser
  participant F as Frontend API Proxy
  participant A as orsgraph-api
  participant G as Neo4j Graph
  participant S as Railway Bucket S3 API

  B->>F: POST /api/ors/matters/{matter}/files/uploads
  F->>A: Forward authenticated request
  A->>A: Validate direct cap, API metadata, path, matter access
  A->>G: Create pending CaseDocument
  A->>S: CreateMultipartUpload(object_key)
  S-->>A: provider_upload_id
  A->>G: Create private UploadSession linked to matter/document
  A->>S: Presign UploadPart URLs for first parts
  A-->>F: mode=multipart, part_size_bytes, total_parts, part URLs
  F-->>B: Multipart intent

  loop Up to 3 concurrent parts
    B->>S: PUT signed part URL with file slice
    S-->>B: ETag
  end

  alt Resume after refresh or retry
    B->>F: GET /api/ors/.../uploads/{upload}/parts
    F->>A: Forward
    A->>S: ListParts(provider_upload_id)
    S-->>A: Uploaded part numbers + ETags
    A-->>B: Uploaded parts
    B->>F: POST /api/ors/.../uploads/{upload}/parts
    F->>A: Requested missing part numbers
    A->>S: Presign replacement part URLs
    A-->>B: Fresh part URLs
  end

  B->>F: POST /api/ors/.../uploads/{upload}/complete with ordered ETags
  F->>A: Forward
  A->>A: Validate all parts are present and ordered
  A->>S: CompleteMultipartUpload(provider_upload_id, parts)
  S-->>A: Final object metadata
  A->>G: Mark UploadSession complete and CaseDocument stored
  A-->>B: Stored CaseDocument
```

## Storage Records

```mermaid
erDiagram
  CaseMatter ||--o{ CaseDocument : "HAS_DOCUMENT"
  CaseMatter ||--o{ UploadSession : "HAS_UPLOAD_SESSION"
  CaseDocument ||--o{ UploadSession : "HAS_UPLOAD_SESSION"
  CaseDocument ||--o{ DocumentVersion : "HAS_VERSION"
  CaseDocument ||--o{ SourceSpan : "HAS_SOURCE_SPAN"
  CaseDocument ||--o{ CaseFact : "PROPOSES_FACT"
  CaseDocument ||--o{ TimelineSuggestion : "PRODUCES_TIMELINE_SUGGESTION"
  CaseDocument ||--o{ ObjectStoreObject : "storage_key"
  UploadSession ||--o{ ObjectStoreMultipartUpload : "provider_upload_id"

  CaseMatter {
    string matter_id
    string owner_user_id
  }

  CaseDocument {
    string document_id
    string matter_id
    string storage_key
    string storage_status
    string processing_status
    string original_relative_path
    int bytes
    string mime_type
  }

  UploadSession {
    string upload_session_id
    string upload_id
    string document_id
    string mode
    string status
    string storage_key
    string provider_upload_id
    int part_size_bytes
    int total_parts
    datetime expires_at
  }

  ObjectStoreObject {
    string bucket
    string key
    string etag
    int bytes
  }

  ObjectStoreMultipartUpload {
    string upload_id
    int part_number
    string etag
  }
```

## Limits and Routing

```mermaid
flowchart LR
  A["Legacy API/body upload path"] --> B["ORS_CASEBUILDER_API_UPLOAD_MAX_BYTES = 50 MiB"]
  C["Browser direct upload path"] --> D["ORS_CASEBUILDER_DIRECT_UPLOAD_MAX_BYTES = 20 GiB"]
  D --> E{"File size"}
  E -- "<= 100 MiB" --> F["single signed PUT"]
  E -- "> 100 MiB" --> G["multipart upload"]
  G --> H["64 MiB parts"]
  H --> I["20 GiB = 320 parts"]
```

## Operational Notes

- Browser direct upload requires bucket CORS for the production frontend origin.
- Required bucket CORS methods: `GET`, `HEAD`, `PUT`.
- Required exposed response header: `ETag`.
- Multipart upload ids stay in private `UploadSession` records and are not exposed through normal document lists.
- Canceling a multipart row aborts in-flight browser requests and calls `DELETE /matters/{matter_id}/files/uploads/{upload_id}`.
- Non-Markdown files are still stored and viewable, but remain `view_only` until a dedicated parser/transcription/OCR adapter processes them.
