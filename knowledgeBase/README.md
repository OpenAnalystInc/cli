# OpenAnalyst Knowledge Base Pipeline

Pipeline tools for building and updating the knowledge base that powers the `/knowledge` command.

## What This Contains

- `tools/transcript_kb/` — Core KB engine: document discovery, chunking (SRT/PDF/text), hybrid search (FTS5 + vector), answer synthesis
- `tools/catalog_jobs.py` — Catalog management: folder/file tracking, source resolution, transcript verification
- `tools/transcribe_mega_folder.py` — Remote GPU transcription runner (Whisper + OCR)
- `tools/llm_judge.py` — LLM-based content quality assessment
- `tools/mega_prefetch.py` — MEGA Cloud Drive file prefetching
- `docs/` — Operator guides, runbooks, fleet operations

## Usage

### 1. Sync transcripts into the KB

```bash
pip install -r requirements-knowledge-base.txt

python -c "
from tools.transcript_kb import TranscriptKnowledgeBase
kb = TranscriptKnowledgeBase(transcriptions_root='/path/to/transcriptions')
result = kb.sync()
print(result)
"
```

### 2. Migrate KB to Neo4j (AWS)

```bash
cd ../backend
python migrations/sqlite_to_neo4j.py \
  --sqlite-db /path/to/catalog/transcript_knowledge_base.db \
  --with-embeddings
```

### 3. Query the KB locally

```bash
python -c "
from tools.transcript_kb import TranscriptKnowledgeBase
kb = TranscriptKnowledgeBase()
result = kb.query('ads strategy for D2C', limit=5, synthesize=True)
for r in result['results']:
    print(f'{r[\"score\"]:.3f} | {r[\"breadcrumb\"]}')
if result.get('answer', {}).get('text'):
    print(result['answer']['text'])
"
```

## Data is NOT stored here

This folder contains pipeline code only. Transcription data, SQLite databases, and vector indexes are stored separately and should not be committed.
