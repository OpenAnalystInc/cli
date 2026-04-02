"""Migrate data from SQLite KB to Neo4j graph database.

Reads from transcript_knowledge_base.db and writes to Neo4j,
preserving the exact same schema as nodes and relationships.

Usage:
  # Set env vars first (or use .env file)
  export NEO4J_URI=bolt://your-ec2:7687
  export NEO4J_PASSWORD=your-password
  export SQLITE_KB_DB=/path/to/transcript_knowledge_base.db

  python migrations/sqlite_to_neo4j.py
  python migrations/sqlite_to_neo4j.py --with-embeddings   # also migrate vectors
  python migrations/sqlite_to_neo4j.py --batch-size 500    # adjust batch size
"""

from __future__ import annotations

import argparse
import sqlite3
import sys
import time
from pathlib import Path

# Add parent to path for imports
sys.path.insert(0, str(Path(__file__).parent.parent))

from config import settings


def connect_sqlite(db_path: str) -> sqlite3.Connection:
    conn = sqlite3.connect(db_path)
    conn.row_factory = sqlite3.Row
    return conn


def connect_neo4j():
    from neo4j import GraphDatabase

    driver = GraphDatabase.driver(
        settings.neo4j_uri,
        auth=(settings.neo4j_user, settings.neo4j_password),
    )
    return driver


def ensure_neo4j_schema(driver) -> None:
    """Create all constraints and indexes."""
    from services.neo4j_backend import Neo4jKBBackend

    backend = Neo4jKBBackend()
    backend._driver = driver
    backend.ensure_schema()
    print("[+] Neo4j schema created/verified.")


def migrate_documents(sqlite_conn: sqlite3.Connection, neo4j_driver, batch_size: int) -> int:
    """Migrate kb_documents → Document nodes + Course nodes."""
    rows = sqlite_conn.execute("SELECT * FROM kb_documents").fetchall()
    total = len(rows)
    print(f"[*] Migrating {total} documents...")

    count = 0
    with neo4j_driver.session(database=settings.neo4j_database) as session:
        for i in range(0, total, batch_size):
            batch = rows[i : i + batch_size]
            for row in batch:
                doc = dict(row)
                session.run(
                    "MERGE (co:Course {name: $course_name})",
                    course_name=doc["course_name"],
                )
                session.run(
                    """
                    MERGE (d:Document {document_id: $document_id})
                    SET d.canonical_remote_path = $canonical_remote_path,
                        d.relative_path = $relative_path,
                        d.folder_path = $folder_path,
                        d.media_path = $media_path,
                        d.transcript_path = $transcript_path,
                        d.course_name = $course_name,
                        d.module_path = $module_path,
                        d.lesson_title = $lesson_title,
                        d.breadcrumb = $breadcrumb,
                        d.content_type = $content_type,
                        d.media_kind = $media_kind,
                        d.quality_class = $quality_class,
                        d.content_hash = $content_hash,
                        d.schema_version = $schema_version,
                        d.indexed_at = $indexed_at
                    WITH d
                    MATCH (co:Course {name: $course_name})
                    MERGE (co)-[:HAS_DOCUMENT]->(d)
                    """,
                    **doc,
                )
                count += 1

            print(f"    {min(i + batch_size, total)}/{total} documents", end="\r")

    print(f"\n[+] {count} documents migrated.")
    return count


def migrate_representations(sqlite_conn: sqlite3.Connection, neo4j_driver, batch_size: int) -> int:
    """Migrate kb_representations → Representation nodes."""
    rows = sqlite_conn.execute("SELECT * FROM kb_representations").fetchall()
    total = len(rows)
    print(f"[*] Migrating {total} representations...")

    count = 0
    with neo4j_driver.session(database=settings.neo4j_database) as session:
        for i in range(0, total, batch_size):
            batch = rows[i : i + batch_size]
            for row in batch:
                rep = dict(row)
                # Don't store full text in Neo4j representation nodes (save space)
                rep_text_len = len(rep.get("text", ""))
                rep.pop("text", None)
                rep["text_length"] = rep_text_len

                session.run(
                    """
                    MERGE (r:Representation {representation_id: $representation_id})
                    SET r.path = $path,
                        r.remote_path = $remote_path,
                        r.representation_type = $representation_type,
                        r.content_type = $content_type,
                        r.media_kind = $media_kind,
                        r.quality_class = $quality_class,
                        r.is_primary = $is_primary,
                        r.text_length = $text_length
                    WITH r
                    MATCH (d:Document {document_id: $document_id})
                    MERGE (d)-[:HAS_REPRESENTATION]->(r)
                    """,
                    **rep,
                )
                count += 1

            print(f"    {min(i + batch_size, total)}/{total} representations", end="\r")

    print(f"\n[+] {count} representations migrated.")
    return count


def migrate_chunks(sqlite_conn: sqlite3.Connection, neo4j_driver, batch_size: int) -> int:
    """Migrate kb_chunks → Chunk nodes."""
    rows = sqlite_conn.execute("SELECT * FROM kb_chunks").fetchall()
    total = len(rows)
    print(f"[*] Migrating {total} chunks...")

    count = 0
    with neo4j_driver.session(database=settings.neo4j_database) as session:
        for i in range(0, total, batch_size):
            batch = rows[i : i + batch_size]
            for row in batch:
                chunk = dict(row)
                session.run(
                    """
                    MERGE (c:Chunk {chunk_id: $chunk_id})
                    SET c.text = $text,
                        c.token_count = $token_count,
                        c.content_type = $content_type,
                        c.quality_class = $quality_class,
                        c.course_name = $course_name,
                        c.module_path = $module_path,
                        c.lesson_title = $lesson_title,
                        c.breadcrumb = $breadcrumb,
                        c.has_timestamps = $has_timestamps,
                        c.start_sec = $start_sec,
                        c.end_sec = $end_sec,
                        c.page_start = $page_start,
                        c.page_end = $page_end,
                        c.chunk_index = $chunk_index,
                        c.indexed_at = $indexed_at
                    WITH c
                    MATCH (d:Document {document_id: $document_id})
                    MERGE (d)-[:HAS_CHUNK]->(c)
                    """,
                    **chunk,
                )
                count += 1

            print(f"    {min(i + batch_size, total)}/{total} chunks", end="\r")

    print(f"\n[+] {count} chunks migrated.")
    return count


def migrate_embeddings(sqlite_conn: sqlite3.Connection, neo4j_driver, qdrant_path: str, batch_size: int) -> int:
    """Migrate vector embeddings from Qdrant to Neo4j vector index."""
    try:
        from qdrant_client import QdrantClient

        client = QdrantClient(path=qdrant_path)
        collections = {c.name for c in client.get_collections().collections}
        if "transcript_chunks" not in collections:
            print("[!] No transcript_chunks collection in Qdrant. Skipping embeddings.")
            return 0

        total = client.count("transcript_chunks", exact=True).count
        print(f"[*] Migrating {total} embeddings from Qdrant...")

        count = 0
        offset = None
        with neo4j_driver.session(database=settings.neo4j_database) as session:
            while True:
                points, offset = client.scroll(
                    "transcript_chunks",
                    limit=batch_size,
                    offset=offset,
                    with_vectors=True,
                )
                if not points:
                    break

                for point in points:
                    chunk_id = str(point.id)
                    vector = point.vector
                    if vector:
                        session.run(
                            "MATCH (c:Chunk {chunk_id: $chunk_id}) SET c.embedding = $embedding",
                            chunk_id=chunk_id,
                            embedding=list(vector),
                        )
                        count += 1

                print(f"    {count}/{total} embeddings", end="\r")

                if offset is None:
                    break

        print(f"\n[+] {count} embeddings migrated.")
        return count
    except ImportError:
        print("[!] qdrant_client not installed. Skipping embeddings.")
        return 0
    except Exception as exc:
        print(f"[!] Embedding migration failed: {exc}")
        return 0


def main():
    parser = argparse.ArgumentParser(description="Migrate SQLite KB to Neo4j")
    parser.add_argument("--with-embeddings", action="store_true", help="Also migrate vector embeddings from Qdrant")
    parser.add_argument("--batch-size", type=int, default=200, help="Batch size for Neo4j transactions")
    parser.add_argument("--sqlite-db", type=str, default=None, help="Path to transcript_knowledge_base.db")
    parser.add_argument("--qdrant-path", type=str, default=None, help="Path to Qdrant data directory")
    args = parser.parse_args()

    db_path = args.sqlite_db or settings.sqlite_kb_db
    if not db_path:
        print("[!] Set SQLITE_KB_DB env var or pass --sqlite-db")
        sys.exit(1)

    if not Path(db_path).exists():
        print(f"[!] SQLite DB not found: {db_path}")
        sys.exit(1)

    print(f"[*] Source:  {db_path}")
    print(f"[*] Target:  {settings.neo4j_uri}")
    print(f"[*] Database: {settings.neo4j_database}")
    print()

    sqlite_conn = connect_sqlite(db_path)
    neo4j_driver = connect_neo4j()

    start = time.time()

    # Verify Neo4j connection
    try:
        with neo4j_driver.session(database=settings.neo4j_database) as session:
            session.run("RETURN 1")
        print("[+] Neo4j connection verified.")
    except Exception as exc:
        print(f"[!] Cannot connect to Neo4j: {exc}")
        sys.exit(1)

    # Create schema
    ensure_neo4j_schema(neo4j_driver)

    # Migrate data
    doc_count = migrate_documents(sqlite_conn, neo4j_driver, args.batch_size)
    rep_count = migrate_representations(sqlite_conn, neo4j_driver, args.batch_size)
    chunk_count = migrate_chunks(sqlite_conn, neo4j_driver, args.batch_size)

    embed_count = 0
    if args.with_embeddings:
        qdrant_path = args.qdrant_path or settings.sqlite_qdrant_path
        if qdrant_path:
            embed_count = migrate_embeddings(sqlite_conn, neo4j_driver, qdrant_path, args.batch_size)
        else:
            print("[!] Set SQLITE_QDRANT_PATH or pass --qdrant-path for embedding migration")

    elapsed = time.time() - start

    print(f"\n{'=' * 50}")
    print(f"Migration complete in {elapsed:.1f}s")
    print(f"  Documents:       {doc_count}")
    print(f"  Representations: {rep_count}")
    print(f"  Chunks:          {chunk_count}")
    print(f"  Embeddings:      {embed_count}")
    print(f"{'=' * 50}")

    sqlite_conn.close()
    neo4j_driver.close()


if __name__ == "__main__":
    main()
