"""LLM answer synthesis — shared between SQLite and Neo4j backends."""

from __future__ import annotations

from typing import Any

from config import settings


def synthesize_answer(query_text: str, results: list[dict[str, Any]]) -> dict[str, Any]:
    """Synthesize a final answer from search results using the configured LLM provider."""
    if not results:
        return {"text": None, "available": False, "reason": "No results to synthesize from."}

    prompt = _build_prompt(query_text, results)

    provider = settings.synthesis_provider
    try:
        if provider == "gemini":
            return _synthesize_gemini(prompt)
        elif provider == "openai":
            return _synthesize_openai(prompt)
        elif provider == "anthropic":
            return _synthesize_anthropic(prompt)
        else:
            return {"text": None, "available": False, "reason": f"Unknown synthesis provider: {provider}"}
    except Exception as exc:
        return {"text": None, "available": False, "reason": f"Synthesis failed: {exc}"}


def _build_prompt(query_text: str, results: list[dict[str, Any]]) -> str:
    context_lines = []
    for i, result in enumerate(results, start=1):
        citation = result.get("citation_label", "")
        breadcrumb = result.get("breadcrumb", "")
        text = result.get("text", "")
        graph_tag = " [graph-expanded]" if result.get("_graph_expanded") else ""
        context_lines.append(f"[{i}] {breadcrumb} ({citation}){graph_tag}\n{text}\n")

    context = "\n".join(context_lines)
    return (
        "You are an expert consultant. Answer the question using the provided knowledge base context. "
        "Be specific, actionable, and practical. Quote citations inline like [1], [2]. "
        "If multiple sources agree, synthesize them. If context is insufficient, say so.\n\n"
        f"Question: {query_text}\n\nContext:\n{context}"
    )


def _synthesize_gemini(prompt: str) -> dict[str, Any]:
    api_key = settings.gemini_api_key
    if not api_key:
        return {"text": None, "available": False, "reason": "GEMINI_API_KEY not configured."}

    import google.generativeai as genai

    genai.configure(api_key=api_key)
    model = genai.GenerativeModel(settings.synthesis_model_resolved)
    response = model.generate_content(prompt)
    text = getattr(response, "text", None) or ""
    return {"text": text.strip() or None, "available": bool(text.strip()), "reason": None}


def _synthesize_openai(prompt: str) -> dict[str, Any]:
    api_key = settings.openai_api_key
    if not api_key:
        return {"text": None, "available": False, "reason": "OPENAI_API_KEY not configured."}

    from openai import OpenAI

    client = OpenAI(api_key=api_key)
    response = client.chat.completions.create(
        model=settings.synthesis_model_resolved,
        messages=[{"role": "user", "content": prompt}],
        max_tokens=2048,
    )
    text = response.choices[0].message.content or ""
    return {"text": text.strip() or None, "available": bool(text.strip()), "reason": None}


def _synthesize_anthropic(prompt: str) -> dict[str, Any]:
    api_key = settings.anthropic_api_key
    if not api_key:
        return {"text": None, "available": False, "reason": "ANTHROPIC_API_KEY not configured."}

    import anthropic

    client = anthropic.Anthropic(api_key=api_key)
    response = client.messages.create(
        model=settings.synthesis_model_resolved,
        max_tokens=2048,
        messages=[{"role": "user", "content": prompt}],
    )
    text = response.content[0].text if response.content else ""
    return {"text": text.strip() or None, "available": bool(text.strip()), "reason": None}
