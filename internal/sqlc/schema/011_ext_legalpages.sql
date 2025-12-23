-- Legal pages extension tables

CREATE TABLE IF NOT EXISTS ext_legalpages_legal_documents (
    id TEXT PRIMARY KEY,
    document_type TEXT NOT NULL,
    title TEXT NOT NULL,
    content TEXT,
    version INTEGER NOT NULL DEFAULT 1,
    status TEXT DEFAULT 'draft',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by TEXT
);

CREATE INDEX IF NOT EXISTS idx_ext_legalpages_legal_documents_doc_type_status ON ext_legalpages_legal_documents(document_type, status);
