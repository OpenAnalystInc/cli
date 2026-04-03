#!/usr/bin/env python3
"""LLM-as-judge creative scoring for generated videos using Gemini API."""

from __future__ import annotations

import json
import os
import time
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Any

ROOT_DIR = Path(__file__).resolve().parents[1]
ASSETS_DIR = ROOT_DIR / "assets"

# ---------------------------------------------------------------------------
# Scoring rubric definitions
# ---------------------------------------------------------------------------

SCORING_DIMENSIONS: dict[str, list[dict[str, Any]]] = {
    "product_ad": [
        {
            "name": "hook_strength",
            "weight": 0.25,
            "description": (
                "How effectively does the opening grab attention? "
                "1-3: no visual hook. 4-6: weak opener. 7-8: clear attention builder. "
                "9-10: impossible to scroll past."
            ),
        },
        {
            "name": "subject_consistency",
            "weight": 0.20,
            "description": (
                "Does the main subject (person/product) maintain identity across all frames? "
                "1-3: subject morphs or disappears. 4-6: minor face/body drift. "
                "7-8: consistent with minor wobble. 9-10: perfect identity lock."
            ),
        },
        {
            "name": "text_readability",
            "weight": 0.15,
            "description": (
                "Is any on-screen text clear and legible? "
                "1-3: garbled or unreadable. 4-6: partially readable. "
                "7-8: clear and readable. 9-10: crisp, styled, perfectly placed. "
                "If no text is expected, score 8."
            ),
        },
        {
            "name": "brand_safety",
            "weight": 0.15,
            "description": (
                "Is the content brand-safe and professional? "
                "1-3: offensive or inappropriate. 4-6: ambiguous elements. "
                "7-8: on-brand, professional. 9-10: premium, perfectly aligned."
            ),
        },
        {
            "name": "motion_quality",
            "weight": 0.15,
            "description": (
                "Is the motion smooth and natural? "
                "1-3: jittery, heavy artifacts. 4-6: floating/warping. "
                "7-8: smooth, natural motion. 9-10: cinematic quality."
            ),
        },
        {
            "name": "overall_coherence",
            "weight": 0.10,
            "description": (
                "Does the video match the generation prompt? "
                "1-3: scene does not match. 4-6: partial match. "
                "7-8: good match with minor deviations. 9-10: perfect adherence."
            ),
        },
    ],
    "brand_story": [
        {
            "name": "narrative_coherence",
            "weight": 0.25,
            "description": (
                "Does the video convey a coherent story beat? "
                "1-3: no narrative flow. 4-6: disjointed. "
                "7-8: clear story beat. 9-10: compelling narrative."
            ),
        },
        {
            "name": "emotional_impact",
            "weight": 0.25,
            "description": (
                "Does the visual mood evoke the intended emotion? "
                "1-3: emotionally flat. 4-6: weak emotional signal. "
                "7-8: clear emotional resonance. 9-10: deeply moving."
            ),
        },
        {
            "name": "visual_quality",
            "weight": 0.20,
            "description": (
                "Resolution, color grading, lighting consistency. "
                "1-3: low quality, artifacts. 4-6: passable with issues. "
                "7-8: high quality visuals. 9-10: cinematic grade."
            ),
        },
        {
            "name": "pacing",
            "weight": 0.15,
            "description": (
                "Does the motion timing match the story beat rhythm? "
                "1-3: awkward timing. 4-6: uneven pacing. "
                "7-8: well-paced. 9-10: masterful rhythm."
            ),
        },
        {
            "name": "brand_alignment",
            "weight": 0.15,
            "description": (
                "Does the visual style fit the brand identity? "
                "1-3: off-brand. 4-6: generic. "
                "7-8: on-brand. 9-10: perfectly brand-embodied."
            ),
        },
    ],
    "ugc": [
        {
            "name": "authenticity",
            "weight": 0.25,
            "description": (
                "Does it look like real user-generated content? "
                "1-3: obviously artificial. 4-6: staged feeling. "
                "7-8: believable UGC. 9-10: indistinguishable from real."
            ),
        },
        {
            "name": "speaking_clarity",
            "weight": 0.20,
            "description": (
                "If there is a speaking subject, is lip sync plausible? "
                "1-3: no lip movement or wildly off. 4-6: visible desync. "
                "7-8: plausible sync. 9-10: natural speaking. "
                "If no speaking expected, score 7."
            ),
        },
        {
            "name": "camera_stability",
            "weight": 0.20,
            "description": (
                "Is handheld camera movement natural? "
                "1-3: jittery/artificial shake. 4-6: unnatural movement. "
                "7-8: natural handheld drift. 9-10: perfectly authentic movement."
            ),
        },
        {
            "name": "natural_feel",
            "weight": 0.20,
            "description": (
                "Does lighting, setting, and subject behavior feel unstaged? "
                "1-3: obviously fake. 4-6: slightly off. "
                "7-8: natural feel. 9-10: completely authentic."
            ),
        },
        {
            "name": "engagement",
            "weight": 0.15,
            "description": (
                "Would a viewer keep watching past the first 2 seconds? "
                "1-3: immediately skip. 4-6: mild interest. "
                "7-8: engaging. 9-10: can't look away."
            ),
        },
    ],
}

DEFAULT_THRESHOLDS: dict[str, dict[str, float]] = {
    "product_ad": {"pass": 7.0, "acceptable": 5.0},
    "brand_story": {"pass": 7.0, "acceptable": 5.0},
    "ugc": {"pass": 6.5, "acceptable": 5.0},
}


# ---------------------------------------------------------------------------
# Dataclasses
# ---------------------------------------------------------------------------


@dataclass
class DimensionScore:
    name: str
    score: float  # 1-10
    feedback: str  # specific textual feedback
    confidence: float  # 0.0-1.0


@dataclass
class JudgeVerdict:
    variant_id: str
    use_case: str
    overall_score: float
    weighted_score: float
    dimensions: list[DimensionScore]
    pass_threshold: float
    passed: bool
    summary: str
    improvement_suggestions: list[str]
    raw_response: str
    model_used: str
    duration_s: float


@dataclass
class PromptRewrite:
    original_prompt: str
    rewritten_prompt: str
    changes_made: list[str]
    judge_feedback_used: str
    model_used: str


# ---------------------------------------------------------------------------
# Errors
# ---------------------------------------------------------------------------


class JudgeError(Exception):
    """Raised when LLM judge encounters an operational error."""


# ---------------------------------------------------------------------------
# LlmJudge
# ---------------------------------------------------------------------------


class LlmJudge:
    """Gemini-based creative quality judge for generated videos."""

    def __init__(
        self,
        gemini_api_key: str | None = None,
        model_name: str = "gemini-2.0-flash",
        assets_dir: Path = ASSETS_DIR,
    ) -> None:
        self.assets_dir = assets_dir
        self.model_name = model_name
        self._api_key = gemini_api_key
        self._genai: Any = None

    @classmethod
    def from_credentials(cls, assets_dir: Path = ASSETS_DIR) -> "LlmJudge":
        """Factory: load API key from env var or assets/gemini_api_key.txt."""
        api_key = _load_gemini_api_key(assets_dir)
        return cls(gemini_api_key=api_key, assets_dir=assets_dir)

    # ------------------------------------------------------------------
    # Internal helpers
    # ------------------------------------------------------------------

    def _ensure_genai(self) -> Any:
        """Lazy-import and configure google.generativeai."""
        if self._genai is not None:
            return self._genai
        try:
            import google.generativeai as genai  # type: ignore
        except ModuleNotFoundError as exc:
            raise JudgeError(
                "LLM judge requires google-generativeai. "
                "Install with: pip install google-generativeai"
            ) from exc
        api_key = self._api_key or _load_gemini_api_key(self.assets_dir)
        genai.configure(api_key=api_key)
        self._genai = genai
        return genai

    def _upload_video(self, video_path: Path, timeout_s: int = 120) -> Any:
        """Upload video to Gemini Files API and wait for ACTIVE state."""
        genai = self._ensure_genai()
        video_file = genai.upload_file(path=str(video_path))
        start = time.time()
        while video_file.state.name == "PROCESSING":
            if time.time() - start > timeout_s:
                raise JudgeError(
                    f"Video upload timed out after {timeout_s}s. "
                    f"State: {video_file.state.name}"
                )
            time.sleep(2)
            video_file = genai.get_file(video_file.name)
        if video_file.state.name != "ACTIVE":
            raise JudgeError(
                f"Video upload failed. State: {video_file.state.name}"
            )
        return video_file

    # ------------------------------------------------------------------
    # Prompt builders
    # ------------------------------------------------------------------

    def _build_scoring_prompt(self, use_case: str, original_prompt: str) -> str:
        """Build the rubric-specific scoring prompt for Gemini."""
        dimensions = SCORING_DIMENSIONS.get(use_case)
        if not dimensions:
            raise JudgeError(
                f"Unknown use_case '{use_case}'. "
                f"Supported: {sorted(SCORING_DIMENSIONS.keys())}"
            )

        dimension_lines = []
        for dim in dimensions:
            dimension_lines.append(
                f"- **{dim['name']}** (weight: {dim['weight']}): {dim['description']}"
            )
        dimensions_text = "\n".join(dimension_lines)

        return f"""\
You are a professional video creative director reviewing an AI-generated video.

The video was generated from this prompt:
---
{original_prompt}
---

Score the video on these dimensions (1-10 scale each):

{dimensions_text}

Return ONLY valid JSON (no markdown fences, no commentary) with this exact structure:
{{
  "dimensions": [
    {{
      "name": "<dimension_name>",
      "score": <1-10>,
      "feedback": "<specific feedback for this dimension>",
      "confidence": <0.0-1.0>
    }}
  ],
  "overall_score": <simple average of all scores>,
  "summary": "<2-3 sentence overall assessment>",
  "improvement_suggestions": ["<specific actionable suggestion>", "..."]
}}

Be honest and precise. A score of 7+ means the dimension is genuinely good.
A score below 5 means there are clear problems. Do not inflate scores."""

    def _build_rewrite_prompt(
        self, original_prompt: str, verdict: JudgeVerdict, use_case: str
    ) -> str:
        """Build the prompt-rewrite request for Gemini."""
        failing_dims = [d for d in verdict.dimensions if d.score < 7.0]
        failing_lines = []
        for dim in failing_dims:
            failing_lines.append(
                f"  - {dim.name}: {dim.score}/10 — {dim.feedback}"
            )
        failing_text = "\n".join(failing_lines) if failing_lines else "  (none below 7)"

        suggestions_text = "\n".join(
            f"  - {s}" for s in verdict.improvement_suggestions
        )

        return f"""\
You are a video generation prompt engineer. A video was generated from the
original prompt below and an AI judge found quality issues.

ORIGINAL PROMPT:
{original_prompt}

JUDGE FEEDBACK:
Overall score: {verdict.weighted_score:.1f}/10 (threshold: {verdict.pass_threshold})
Failing dimensions:
{failing_text}

IMPROVEMENT SUGGESTIONS:
{suggestions_text}

Rewrite the prompt to address these specific issues. Rules:
1. Keep the same scene concept, characters, and setting
2. Add or strengthen language that addresses each failing dimension
3. Keep it as a single flowing paragraph
4. Must include camera language, mood/lighting cues, audio cues, and chronological markers
5. Do not add new characters or change the fundamental scene
6. Do not include any meta-instructions — output only the scene description

Return ONLY valid JSON (no markdown fences, no commentary):
{{
  "rewritten_prompt": "<the improved prompt>",
  "changes_made": ["<specific change 1>", "<specific change 2>"]
}}"""

    # ------------------------------------------------------------------
    # Response parsers
    # ------------------------------------------------------------------

    @staticmethod
    def _extract_json(raw: str) -> dict[str, Any]:
        """Parse JSON from model output, stripping markdown fences if present."""
        text = raw.strip()
        # Strip markdown code fences
        if "```json" in text:
            text = text.split("```json")[1].split("```")[0]
        elif "```" in text:
            text = text.split("```")[1].split("```")[0]
        return json.loads(text.strip())

    def _parse_judge_response(
        self,
        raw: str,
        variant_id: str,
        use_case: str,
        original_prompt: str,
        thresholds: dict[str, float],
        duration_s: float,
    ) -> JudgeVerdict:
        """Parse Gemini's scoring response into a JudgeVerdict."""
        try:
            data = self._extract_json(raw)
        except (json.JSONDecodeError, IndexError) as exc:
            # Parse failure → treated as non-passing
            return JudgeVerdict(
                variant_id=variant_id,
                use_case=use_case,
                overall_score=0.0,
                weighted_score=0.0,
                dimensions=[],
                pass_threshold=thresholds.get("pass", 7.0),
                passed=False,
                summary=f"Judge response parse error: {exc}",
                improvement_suggestions=["Re-run the judge — response was not valid JSON."],
                raw_response=raw[:2000],
                model_used=self.model_name,
                duration_s=duration_s,
            )

        dimensions_raw = data.get("dimensions", [])
        dimension_scores: list[DimensionScore] = []
        for item in dimensions_raw:
            dimension_scores.append(
                DimensionScore(
                    name=str(item.get("name", "unknown")),
                    score=float(item.get("score", 0)),
                    feedback=str(item.get("feedback", "")),
                    confidence=float(item.get("confidence", 0.5)),
                )
            )

        overall_score = float(data.get("overall_score", 0))

        # Compute weighted score from dimensions
        dims_lookup = SCORING_DIMENSIONS.get(use_case, [])
        weight_map = {d["name"]: d["weight"] for d in dims_lookup}
        weighted_score = 0.0
        total_weight = 0.0
        for ds in dimension_scores:
            w = weight_map.get(ds.name, 0.0)
            weighted_score += ds.score * w
            total_weight += w
        if total_weight > 0:
            weighted_score = weighted_score / total_weight
        else:
            weighted_score = overall_score

        pass_threshold = thresholds.get("pass", 7.0)

        return JudgeVerdict(
            variant_id=variant_id,
            use_case=use_case,
            overall_score=overall_score,
            weighted_score=round(weighted_score, 2),
            dimensions=dimension_scores,
            pass_threshold=pass_threshold,
            passed=weighted_score >= pass_threshold,
            summary=str(data.get("summary", "")),
            improvement_suggestions=list(data.get("improvement_suggestions", [])),
            raw_response=raw[:2000],
            model_used=self.model_name,
            duration_s=duration_s,
        )

    def _parse_rewrite_response(
        self, raw: str, original_prompt: str, verdict: JudgeVerdict
    ) -> PromptRewrite:
        """Parse Gemini's prompt-rewrite response."""
        try:
            data = self._extract_json(raw)
        except (json.JSONDecodeError, IndexError) as exc:
            raise JudgeError(
                f"Could not parse prompt rewrite response: {exc}. Raw: {raw[:500]}"
            ) from exc

        rewritten = data.get("rewritten_prompt", "")
        if not rewritten or not isinstance(rewritten, str):
            raise JudgeError(
                "Prompt rewrite response missing 'rewritten_prompt' field."
            )

        failing_dims = [d for d in verdict.dimensions if d.score < 7.0]
        feedback_summary = "; ".join(
            f"{d.name}: {d.score}/10 — {d.feedback}" for d in failing_dims
        )

        return PromptRewrite(
            original_prompt=original_prompt,
            rewritten_prompt=rewritten,
            changes_made=list(data.get("changes_made", [])),
            judge_feedback_used=feedback_summary,
            model_used=self.model_name,
        )

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------

    def judge_video(
        self,
        video_path: Path,
        use_case: str,
        variant_id: str,
        original_prompt: str,
        thresholds: dict[str, float] | None = None,
    ) -> JudgeVerdict:
        """Upload video to Gemini, run scoring rubric, return structured verdict."""
        if thresholds is None:
            thresholds = DEFAULT_THRESHOLDS.get(use_case, {"pass": 7.0, "acceptable": 5.0})

        video_path = Path(video_path)
        if not video_path.exists():
            raise JudgeError(f"Video file not found: {video_path}")

        genai = self._ensure_genai()
        started = time.time()

        # Upload video
        video_file = self._upload_video(video_path)

        # Build and send scoring prompt
        scoring_prompt = self._build_scoring_prompt(use_case, original_prompt)
        model = genai.GenerativeModel(self.model_name)
        response = model.generate_content([video_file, scoring_prompt])
        raw_text = response.text

        duration_s = round(time.time() - started, 2)

        return self._parse_judge_response(
            raw=raw_text,
            variant_id=variant_id,
            use_case=use_case,
            original_prompt=original_prompt,
            thresholds=thresholds,
            duration_s=duration_s,
        )

    def rewrite_prompt(
        self,
        original_prompt: str,
        verdict: JudgeVerdict,
        use_case: str,
    ) -> PromptRewrite:
        """Use Gemini to rewrite the prompt, addressing judge feedback."""
        genai = self._ensure_genai()

        rewrite_prompt_text = self._build_rewrite_prompt(
            original_prompt, verdict, use_case
        )
        model = genai.GenerativeModel(self.model_name)
        response = model.generate_content(rewrite_prompt_text)
        raw_text = response.text

        return self._parse_rewrite_response(raw_text, original_prompt, verdict)


# ---------------------------------------------------------------------------
# Standalone helpers
# ---------------------------------------------------------------------------


def _read_trimmed(path: Path) -> str:
    """Read a file and strip whitespace + quotes."""
    value = path.read_text(encoding="utf-8").strip()
    return value.strip('"').strip("'")


def _load_gemini_api_key(assets_dir: Path = ASSETS_DIR) -> str:
    """Load Gemini API key from env var or assets/gemini_api_key.txt."""
    api_key = os.getenv("GEMINI_API_KEY")
    if api_key:
        return api_key.strip()
    key_path = assets_dir / "gemini_api_key.txt"
    if key_path.exists():
        value = _read_trimmed(key_path)
        if value:
            return value
    raise JudgeError(
        "Missing GEMINI_API_KEY. Set the environment variable or create "
        "assets/gemini_api_key.txt with your Gemini API key."
    )


def verdict_from_scorecard_creative(
    creative_score: dict[str, Any],
    variant_id: str,
    use_case: str,
) -> JudgeVerdict:
    """Reconstruct a JudgeVerdict from a scorecard's creative_score block."""
    dimensions = [
        DimensionScore(
            name=d["name"],
            score=d["score"],
            feedback=d["feedback"],
            confidence=d.get("confidence", 0.5),
        )
        for d in creative_score.get("dimensions", [])
    ]
    return JudgeVerdict(
        variant_id=variant_id,
        use_case=use_case,
        overall_score=creative_score.get("overall_score", 0.0),
        weighted_score=creative_score.get("weighted_score", 0.0),
        dimensions=dimensions,
        pass_threshold=creative_score.get("pass_threshold", 7.0),
        passed=creative_score.get("passed", False),
        summary=creative_score.get("summary", ""),
        improvement_suggestions=creative_score.get("improvement_suggestions", []),
        raw_response="",
        model_used=creative_score.get("model_used", "unknown"),
        duration_s=creative_score.get("duration_s", 0.0),
    )
