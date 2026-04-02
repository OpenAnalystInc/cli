"""Import transcription catalog data into Neo4j knowledge graph.

Reads from transcription_catalog.db and builds:
  (:Source) -[:HAS_COURSE]-> (:Course) -[:HAS_MODULE]-> (:Module)
  (:Module) -[:CONTAINS]-> (:File)
  (:File) -[:HAS_TRANSCRIPT]-> (:Transcript)
  (:Course) -[:TAGGED]-> (:Tag)

Prepares the graph structure for TRIBE v2 embeddings (added later).

Usage:
  python migrations/import_catalog_to_neo4j.py \
    --catalog-db /path/to/transcription_catalog.db
"""

from __future__ import annotations

import argparse
import re
import sqlite3
import sys
import time
from pathlib import Path, PurePosixPath

sys.path.insert(0, str(Path(__file__).parent.parent))
sys.stdout.reconfigure(encoding='utf-8')

from config import settings


# ── Tag extraction from course names ─────────────────────────────────

TAG_PATTERNS = {
    "AI": r"\bAI\b|artificial intelligence|machine learning|\bLLM\b|GPT|ChatGPT|Claude",
    "Ads": r"\bAds?\b|advertising|PPC|paid media|Meta Ads|Google Ads|Facebook Ads",
    "Marketing": r"marketing|funnel|lead gen|conversion|copywriting|email marketing",
    "E-Commerce": r"e-?commerce|shopify|dropshipping|DTC|D2C|amazon fba",
    "Social Media": r"social media|instagram|tiktok|youtube|pinterest|linkedin|twitter",
    "SEO": r"\bSEO\b|search engine|organic traffic|backlink",
    "Design": r"design|figma|canva|creative|branding|logo",
    "Video": r"video|youtube|vlog|editing|premiere|davinci",
    "Voice AI": r"voice ai|voice agent|speech|TTS|whisper",
    "Automation": r"automation|n8n|zapier|make\.com|workflow",
    "Agency": r"agency|client|freelance|consulting",
    "Finance": r"finance|investing|trading|crypto|stock|wealth",
    "Coaching": r"coaching|course creator|mastermind|mentoring",
    "Personal Brand": r"personal brand|influence|authority|thought leader",
    "Sales": r"sales|closing|cold call|outreach|pipeline",
    "Content": r"content|writing|blog|newsletter|creator",
    "Development": r"coding|programming|python|javascript|react|rust|web dev|software",
    "No-Code": r"no-?code|low-?code|bubble|webflow|cursor|vibe coding",
}


def extract_tags(name: str) -> list[str]:
    tags = []
    for tag, pattern in TAG_PATTERNS.items():
        if re.search(pattern, name, re.IGNORECASE):
            tags.append(tag)
    return tags


def connect_neo4j():
    from neo4j import GraphDatabase
    return GraphDatabase.driver(
        settings.neo4j_uri,
        auth=(settings.neo4j_user, settings.neo4j_password),
    )


def ensure_schema(driver) -> None:
    """Create constraints and indexes for the catalog graph."""
    with driver.session(database=settings.neo4j_database) as session:
        constraints = [
            "CREATE CONSTRAINT source_id IF NOT EXISTS FOR (s:Source) REQUIRE s.source_id IS UNIQUE",
            "CREATE CONSTRAINT course_path IF NOT EXISTS FOR (c:Course) REQUIRE c.path IS UNIQUE",
            "CREATE CONSTRAINT folder_path IF NOT EXISTS FOR (m:Module) REQUIRE m.path IS UNIQUE",
            "CREATE CONSTRAINT file_path IF NOT EXISTS FOR (f:File) REQUIRE f.path IS UNIQUE",
            "CREATE CONSTRAINT tag_name IF NOT EXISTS FOR (t:Tag) REQUIRE t.name IS UNIQUE",
        ]
        indexes = [
            "CREATE INDEX course_name IF NOT EXISTS FOR (c:Course) ON (c.name)",
            "CREATE INDEX course_status IF NOT EXISTS FOR (c:Course) ON (c.status)",
            "CREATE INDEX file_kind IF NOT EXISTS FOR (f:File) ON (f.kind)",
            "CREATE INDEX file_ext IF NOT EXISTS FOR (f:File) ON (f.extension)",
            "CREATE INDEX file_transcript_status IF NOT EXISTS FOR (f:File) ON (f.transcript_status)",
        ]
        for stmt in constraints + indexes:
            try:
                session.run(stmt)
            except Exception:
                pass

        # Fulltext index for search
        try:
            session.run(
                "CREATE FULLTEXT INDEX course_search IF NOT EXISTS "
                "FOR (c:Course) ON EACH [c.name, c.path]"
            )
        except Exception:
            pass
        try:
            session.run(
                "CREATE FULLTEXT INDEX file_search IF NOT EXISTS "
                "FOR (f:File) ON EACH [f.basename, f.path]"
            )
        except Exception:
            pass

    print("[+] Neo4j schema created.")


def import_sources(conn: sqlite3.Connection, driver, batch_size: int) -> int:
    rows = conn.execute("SELECT * FROM sources").fetchall()
    print(f"[*] Importing {len(rows)} sources...")
    count = 0
    with driver.session(database=settings.neo4j_database) as session:
        for row in rows:
            session.run(
                """
                MERGE (s:Source {source_id: $id})
                SET s.display_name = $display_name,
                    s.canonical_path = $canonical_path,
                    s.current_path = $current_path,
                    s.browser_url = $browser_url,
                    s.status = $status,
                    s.last_run_id = $last_run_id,
                    s.resolved_at = $resolved_at,
                    s.updated_at = $updated_at
                """,
                id=row["id"],
                display_name=row["display_name"],
                canonical_path=row["canonical_path"],
                current_path=row["current_path"],
                browser_url=row["browser_url"],
                status=row["status"],
                last_run_id=row["last_run_id"],
                resolved_at=row["resolved_at"],
                updated_at=row["updated_at"],
            )
            count += 1
    print(f"[+] {count} sources imported.")
    return count


def import_courses_and_modules(conn: sqlite3.Connection, driver, batch_size: int) -> tuple[int, int]:
    """Import depth=1 folders as Course nodes, deeper folders as Module nodes."""
    courses = conn.execute(
        "SELECT * FROM folders WHERE depth = 1 ORDER BY name"
    ).fetchall()
    print(f"[*] Importing {len(courses)} courses...")

    course_count = 0
    module_count = 0

    with driver.session(database=settings.neo4j_database) as session:
        for i, row in enumerate(courses):
            tags = extract_tags(row["name"])

            session.run(
                """
                MERGE (c:Course {path: $path})
                SET c.name = $name,
                    c.status = $status,
                    c.depth = $depth,
                    c.media_count = $media_count,
                    c.processed_media_count = $processed_media_count,
                    c.pending_media_count = $pending_media_count,
                    c.failed_media_count = $failed_media_count,
                    c.processable_count = $processable_count,
                    c.created_at_utc = $created_at_utc,
                    c.updated_at = $updated_at
                """,
                path=row["path"],
                name=row["name"],
                status=row["status"],
                depth=row["depth"],
                media_count=row["media_count"],
                processed_media_count=row["processed_media_count"],
                pending_media_count=row["pending_media_count"],
                failed_media_count=row["failed_media_count"],
                processable_count=row["processable_count"],
                created_at_utc=row["created_at_utc"],
                updated_at=row["updated_at"],
            )

            # Link to source if exists
            session.run(
                """
                MATCH (c:Course {path: $path})
                MATCH (s:Source) WHERE s.canonical_path = $path OR s.current_path = $path
                MERGE (s)-[:HAS_COURSE]->(c)
                """,
                path=row["path"],
            )

            # Add tags
            for tag in tags:
                session.run(
                    """
                    MERGE (t:Tag {name: $tag})
                    WITH t
                    MATCH (c:Course {path: $path})
                    MERGE (c)-[:TAGGED]->(t)
                    """,
                    tag=tag,
                    path=row["path"],
                )

            course_count += 1
            if (i + 1) % 50 == 0:
                print(f"    {i + 1}/{len(courses)} courses...", end="\r")

    print(f"[+] {course_count} courses imported.")

    # Import sub-folders as Module nodes
    modules = conn.execute(
        "SELECT * FROM folders WHERE depth > 1 ORDER BY path"
    ).fetchall()
    print(f"[*] Importing {len(modules)} modules...")

    with driver.session(database=settings.neo4j_database) as session:
        for i in range(0, len(modules), batch_size):
            batch = modules[i:i + batch_size]
            for row in batch:
                session.run(
                    """
                    MERGE (m:Module {path: $path})
                    SET m.name = $name,
                        m.status = $status,
                        m.depth = $depth,
                        m.media_count = $media_count,
                        m.processed_media_count = $processed_media_count,
                        m.pending_media_count = $pending_media_count,
                        m.created_at_utc = $created_at_utc,
                        m.updated_at = $updated_at
                    """,
                    path=row["path"],
                    name=row["name"],
                    status=row["status"],
                    depth=row["depth"],
                    media_count=row["media_count"],
                    processed_media_count=row["processed_media_count"],
                    pending_media_count=row["pending_media_count"],
                    created_at_utc=row["created_at_utc"],
                    updated_at=row["updated_at"],
                )

                # Link to parent (Course or Module)
                parent = row["parent_path"]
                if parent:
                    # Try Course first, then Module
                    session.run(
                        """
                        MATCH (m:Module {path: $path})
                        OPTIONAL MATCH (c:Course {path: $parent})
                        OPTIONAL MATCH (pm:Module {path: $parent})
                        WITH m, coalesce(c, pm) AS parent
                        WHERE parent IS NOT NULL
                        MERGE (parent)-[:HAS_MODULE]->(m)
                        """,
                        path=row["path"],
                        parent=parent,
                    )

                module_count += 1

            if (i + batch_size) % 500 == 0 or i + batch_size >= len(modules):
                print(f"    {min(i + batch_size, len(modules))}/{len(modules)} modules...", end="\r")

    print(f"\n[+] {module_count} modules imported.")
    return course_count, module_count


def import_files(conn: sqlite3.Connection, driver, batch_size: int) -> int:
    """Import files as File nodes linked to their parent Course/Module."""
    total = conn.execute("SELECT COUNT(*) FROM files").fetchone()[0]
    print(f"[*] Importing {total:,} files...")

    count = 0
    offset = 0

    while offset < total:
        rows = conn.execute(
            "SELECT * FROM files ORDER BY id LIMIT ? OFFSET ?",
            (batch_size, offset),
        ).fetchall()

        if not rows:
            break

        with driver.session(database=settings.neo4j_database) as session:
            for row in rows:
                session.run(
                    """
                    MERGE (f:File {path: $path})
                    SET f.basename = $basename,
                        f.extension = $extension,
                        f.kind = $kind,
                        f.transcript_path = $transcript_path,
                        f.transcript_status = $transcript_status,
                        f.transcript_processor = $transcript_processor,
                        f.created_at_utc = $created_at_utc,
                        f.modified_at_utc = $modified_at_utc,
                        f.updated_at = $updated_at,
                        f.source_browser_url = $source_browser_url,
                        f.companion_output_path = $companion_output_path,
                        f.last_run_id = $last_run_id
                    """,
                    path=row["path"],
                    basename=row["basename"],
                    extension=row["extension"],
                    kind=row["kind"],
                    transcript_path=row["transcript_path"],
                    transcript_status=row["transcript_status"],
                    transcript_processor=row["transcript_processor"],
                    created_at_utc=row["created_at_utc"],
                    modified_at_utc=row["modified_at_utc"],
                    updated_at=row["updated_at"],
                    source_browser_url=row["source_browser_url"],
                    companion_output_path=row["companion_output_path"],
                    last_run_id=row["last_run_id"],
                )

                # Link to parent folder (Course or Module)
                parent = row["parent_path"]
                if parent:
                    session.run(
                        """
                        MATCH (f:File {path: $path})
                        OPTIONAL MATCH (c:Course {path: $parent})
                        OPTIONAL MATCH (m:Module {path: $parent})
                        WITH f, coalesce(m, c) AS parent
                        WHERE parent IS NOT NULL
                        MERGE (parent)-[:CONTAINS]->(f)
                        """,
                        path=row["path"],
                        parent=parent,
                    )

                count += 1

        offset += batch_size
        print(f"    {min(offset, total):,}/{total:,} files...", end="\r")

    print(f"\n[+] {count:,} files imported.")
    return count


def import_job_runs(conn: sqlite3.Connection, driver, batch_size: int) -> int:
    """Import job runs as JobRun nodes linked to sources."""
    rows = conn.execute("SELECT * FROM job_runs ORDER BY started_at DESC").fetchall()
    print(f"[*] Importing {len(rows)} job runs...")
    count = 0

    with driver.session(database=settings.neo4j_database) as session:
        for row in rows:
            session.run(
                """
                MERGE (j:JobRun {run_id: $run_id})
                SET j.status = $status,
                    j.processed = $processed,
                    j.skipped = $skipped,
                    j.failed = $failed,
                    j.discovered = $discovered,
                    j.started_at = $started_at,
                    j.finished_at = $finished_at,
                    j.worker_name = $worker_name,
                    j.instance_type = $instance_type,
                    j.source_path_before = $source_path_before,
                    j.source_path_after = $source_path_after
                """,
                run_id=row["run_id"],
                status=row["status"],
                processed=row["processed"],
                skipped=row["skipped"],
                failed=row["failed"],
                discovered=row["discovered"],
                started_at=row["started_at"],
                finished_at=row["finished_at"],
                worker_name=row["worker_name"],
                instance_type=row["instance_type"],
                source_path_before=row["source_path_before"],
                source_path_after=row["source_path_after"],
            )

            # Link to source
            if row["source_id"]:
                session.run(
                    """
                    MATCH (j:JobRun {run_id: $run_id})
                    MATCH (s:Source {source_id: $source_id})
                    MERGE (s)-[:HAS_RUN]->(j)
                    """,
                    run_id=row["run_id"],
                    source_id=row["source_id"],
                )
            count += 1

    print(f"[+] {count} job runs imported.")
    return count


def print_summary(driver) -> None:
    with driver.session(database=settings.neo4j_database) as session:
        labels = ["Source", "Course", "Module", "File", "Tag", "JobRun"]
        print("\n" + "=" * 50)
        print("  Neo4j Knowledge Graph Summary")
        print("=" * 50)
        for label in labels:
            n = session.run(f"MATCH (n:{label}) RETURN count(n) AS n").single()["n"]
            print(f"  {label:15s} {n:>10,} nodes")

        rels = session.run(
            "MATCH ()-[r]->() RETURN type(r) AS t, count(r) AS n ORDER BY n DESC"
        )
        print("\n  Relationships:")
        for record in rels:
            print(f"    {record['t']:20s} {record['n']:>10,}")
        print("=" * 50)


def main():
    parser = argparse.ArgumentParser(description="Import catalog to Neo4j")
    parser.add_argument("--catalog-db", type=str, default=None)
    parser.add_argument("--batch-size", type=int, default=200)
    args = parser.parse_args()

    db_path = args.catalog_db or settings.sqlite_catalog_db
    if not db_path:
        print("[!] Set SQLITE_CATALOG_DB or pass --catalog-db")
        sys.exit(1)
    if not Path(db_path).exists():
        print(f"[!] DB not found: {db_path}")
        sys.exit(1)

    print(f"[*] Source:   {db_path}")
    print(f"[*] Target:   {settings.neo4j_uri}")
    print()

    conn = sqlite3.connect(db_path)
    conn.row_factory = sqlite3.Row
    driver = connect_neo4j()

    # Verify connection
    with driver.session(database=settings.neo4j_database) as session:
        session.run("RETURN 1")
    print("[+] Neo4j connected.\n")

    start = time.time()

    ensure_schema(driver)
    import_sources(conn, driver, args.batch_size)
    course_count, module_count = import_courses_and_modules(conn, driver, args.batch_size)
    file_count = import_files(conn, driver, args.batch_size)
    run_count = import_job_runs(conn, driver, args.batch_size)

    elapsed = time.time() - start
    print(f"\nImport completed in {elapsed:.1f}s")
    print_summary(driver)

    conn.close()
    driver.close()


if __name__ == "__main__":
    main()
