//! Smart per-action model router — dynamically selects model + effort per task category.
//!
//! The routing table maps each ActionCategory to a (ModelTier, EffortLevel) pair.
//! Users configure via `/effort <category> <level>` or `/route` to view/edit.
//!
//! Default routing table (when user's model is capable-tier):
//!   explore  → Fast    + Low    (haiku,  1K thinking)
//!   research → Balanced + Medium (sonnet, 8K thinking)
//!   code     → Capable + High   (opus,   32K thinking)
//!   write    → Balanced + Medium (sonnet, 8K thinking)

use std::fmt;

use events::AgentType;

// ── Action categories ──

/// Category of work derived from prompt analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionCategory {
    /// File reading, listing, searching, simple lookups.
    Explore,
    /// Research, planning, architecture, analysis.
    Research,
    /// Code generation, refactoring, bug fixing, implementation.
    Code,
    /// Documentation, commit messages, PR descriptions, explanations.
    Write,
}

impl ActionCategory {
    pub const ALL: [ActionCategory; 4] = [
        Self::Explore,
        Self::Research,
        Self::Code,
        Self::Write,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Explore => "explore",
            Self::Research => "research",
            Self::Code => "code",
            Self::Write => "write",
        }
    }

    /// Parse from user input string.
    #[must_use]
    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "explore" | "exp" | "e" => Some(Self::Explore),
            "research" | "res" | "r" => Some(Self::Research),
            "code" | "c" => Some(Self::Code),
            "write" | "w" | "doc" => Some(Self::Write),
            _ => None,
        }
    }
}

impl fmt::Display for ActionCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ── Model tier ──

/// Model tier for routing decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelTier {
    /// Fast, cheap — for exploration, file reading, simple search.
    Fast,
    /// Balanced — for planning, moderate coding, reviews.
    Balanced,
    /// Capable — for complex coding, architecture, multi-step reasoning.
    Capable,
}

impl ModelTier {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Fast => "fast",
            Self::Balanced => "balanced",
            Self::Capable => "capable",
        }
    }

    #[must_use]
    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "fast" | "f" => Some(Self::Fast),
            "balanced" | "bal" | "b" => Some(Self::Balanced),
            "capable" | "cap" | "full" => Some(Self::Capable),
            _ => None,
        }
    }
}

impl fmt::Display for ModelTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ── Effort level (mirrors tui::app::EffortLevel but lives in the routing layer) ──

/// Thinking effort level — maps to token budgets for extended thinking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffortLevel {
    Low,
    Medium,
    High,
    Max,
}

impl Default for EffortLevel {
    fn default() -> Self {
        Self::Medium
    }
}

impl EffortLevel {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Max => "max",
        }
    }

    /// Thinking budget tokens for Anthropic extended thinking.
    #[must_use]
    pub const fn thinking_budget(self) -> u32 {
        match self {
            Self::Low => 1_024,
            Self::Medium => 8_192,
            Self::High => 32_000,
            Self::Max => 128_000,
        }
    }

    /// Parse from string.
    #[must_use]
    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "low" | "l" | "1" => Some(Self::Low),
            "medium" | "med" | "m" | "2" => Some(Self::Medium),
            "high" | "h" | "3" => Some(Self::High),
            "max" | "x" | "4" => Some(Self::Max),
            _ => None,
        }
    }
}

impl fmt::Display for EffortLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ── Routing profile (per-category config) ──

/// Configuration for a single action category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoutingProfile {
    pub model_tier: ModelTier,
    pub effort: EffortLevel,
}

// ── Routing table ──

/// The full routing table — maps each ActionCategory to a RoutingProfile.
#[derive(Debug, Clone)]
pub struct RoutingTable {
    pub explore: RoutingProfile,
    pub research: RoutingProfile,
    pub code: RoutingProfile,
    pub write: RoutingProfile,
}

impl Default for RoutingTable {
    fn default() -> Self {
        Self {
            explore: RoutingProfile {
                model_tier: ModelTier::Fast,
                effort: EffortLevel::Low,
            },
            research: RoutingProfile {
                model_tier: ModelTier::Balanced,
                effort: EffortLevel::Medium,
            },
            code: RoutingProfile {
                model_tier: ModelTier::Capable,
                effort: EffortLevel::High,
            },
            write: RoutingProfile {
                model_tier: ModelTier::Balanced,
                effort: EffortLevel::Medium,
            },
        }
    }
}

impl RoutingTable {
    /// Get the profile for a category.
    #[must_use]
    pub fn get(&self, category: ActionCategory) -> &RoutingProfile {
        match category {
            ActionCategory::Explore => &self.explore,
            ActionCategory::Research => &self.research,
            ActionCategory::Code => &self.code,
            ActionCategory::Write => &self.write,
        }
    }

    /// Get mutable profile for a category.
    pub fn get_mut(&mut self, category: ActionCategory) -> &mut RoutingProfile {
        match category {
            ActionCategory::Explore => &mut self.explore,
            ActionCategory::Research => &mut self.research,
            ActionCategory::Code => &mut self.code,
            ActionCategory::Write => &mut self.write,
        }
    }

    /// Set effort for a specific category.
    pub fn set_effort(&mut self, category: ActionCategory, effort: EffortLevel) {
        self.get_mut(category).effort = effort;
    }

    /// Set effort for all categories at once (global `/effort <level>`).
    pub fn set_effort_all(&mut self, effort: EffortLevel) {
        for cat in ActionCategory::ALL {
            self.get_mut(cat).effort = effort;
        }
    }

    /// Set model tier for a specific category.
    pub fn set_tier(&mut self, category: ActionCategory, tier: ModelTier) {
        self.get_mut(category).model_tier = tier;
    }

    /// Render a human-readable routing table for display.
    #[must_use]
    pub fn render_table(&self, resolver: &ModelResolver) -> String {
        let mut lines = Vec::new();
        lines.push("┌──────────┬──────────┬────────┬────────────────────────────┐".to_string());
        lines.push("│ Category │ Effort   │ Tier   │ Model                      │".to_string());
        lines.push("├──────────┼──────────┼────────┼────────────────────────────┤".to_string());
        for cat in ActionCategory::ALL {
            let profile = self.get(cat);
            let model = resolver.resolve(profile.model_tier);
            lines.push(format!(
                "│ {:<8} │ {:<8} │ {:<6} │ {:<26} │",
                cat.as_str(),
                profile.effort.as_str(),
                profile.model_tier.as_str(),
                model,
            ));
        }
        lines.push("└──────────┴──────────┴────────┴────────────────────────────┘".to_string());
        lines.join("\n")
    }
}

// ── Model resolver (maps tiers to concrete model names) ──

/// Resolves model tiers to concrete model name strings.
#[derive(Debug, Clone)]
pub struct ModelResolver {
    pub fast_model: String,
    pub balanced_model: String,
    pub capable_model: String,
}

impl ModelResolver {
    /// Create a resolver from the user's configured default model.
    /// Automatically assigns lighter models to lower tiers.
    #[must_use]
    pub fn from_default_model(user_model: &str) -> Self {
        let (fast, balanced, capable) = match classify_model(user_model) {
            ModelTier::Fast => {
                (user_model.to_string(), user_model.to_string(), user_model.to_string())
            }
            ModelTier::Balanced => {
                ("claude-haiku-4-5".to_string(), user_model.to_string(), user_model.to_string())
            }
            ModelTier::Capable => {
                ("claude-haiku-4-5".to_string(), "claude-sonnet-4-6".to_string(), user_model.to_string())
            }
        };
        Self {
            fast_model: fast,
            balanced_model: balanced,
            capable_model: capable,
        }
    }

    /// Resolve a tier to a concrete model name.
    #[must_use]
    pub fn resolve(&self, tier: ModelTier) -> &str {
        match tier {
            ModelTier::Fast => &self.fast_model,
            ModelTier::Balanced => &self.balanced_model,
            ModelTier::Capable => &self.capable_model,
        }
    }
}

// ── Full model router (combines table + resolver) ──

/// Smart model router — combines routing table with model resolver.
/// This is the main entry point used by the orchestrator.
#[derive(Debug, Clone)]
pub struct ModelRouter {
    pub table: RoutingTable,
    pub resolver: ModelResolver,
}

impl ModelRouter {
    /// Create a router with the user's configured model as the capable tier.
    #[must_use]
    pub fn from_default_model(user_model: &str) -> Self {
        Self {
            table: RoutingTable::default(),
            resolver: ModelResolver::from_default_model(user_model),
        }
    }

    /// Select the model for a given agent type (backward compat).
    #[must_use]
    pub fn model_for_agent(&self, agent_type: &AgentType) -> &str {
        let tier = match agent_type {
            AgentType::Explore => ModelTier::Fast,
            AgentType::Plan => ModelTier::Balanced,
            AgentType::Primary | AgentType::General => ModelTier::Capable,
        };
        self.resolver.resolve(tier)
    }

    /// Classify a prompt and return (model_name, effort_budget) from the routing table.
    /// This is the primary method used for smart per-action routing.
    #[must_use]
    pub fn route_prompt(&self, prompt: &str) -> ResolvedRoute {
        let category = classify_prompt(prompt);
        let profile = self.table.get(category);
        ResolvedRoute {
            category,
            model: self.resolver.resolve(profile.model_tier).to_string(),
            effort_budget: profile.effort.thinking_budget(),
        }
    }

    /// Route for a specific agent type + task (used by sub-agent spawning).
    #[must_use]
    pub fn route_agent_task(&self, agent_type: &AgentType, task: &str) -> ResolvedRoute {
        // Sub-agents use agent-type-based routing, not prompt classification
        let category = match agent_type {
            AgentType::Explore => ActionCategory::Explore,
            AgentType::Plan => ActionCategory::Research,
            AgentType::Primary | AgentType::General => classify_prompt(task),
        };
        let profile = self.table.get(category);
        ResolvedRoute {
            category,
            model: self.resolver.resolve(profile.model_tier).to_string(),
            effort_budget: profile.effort.thinking_budget(),
        }
    }

    /// Render the current routing table for display.
    #[must_use]
    pub fn render_table(&self) -> String {
        self.table.render_table(&self.resolver)
    }
}

/// Result of routing a prompt through the table.
#[derive(Debug, Clone)]
pub struct ResolvedRoute {
    pub category: ActionCategory,
    pub model: String,
    pub effort_budget: u32,
}

// ── Prompt classifier ──

/// Classify a prompt into an ActionCategory based on heuristic patterns.
#[must_use]
pub fn classify_prompt(prompt: &str) -> ActionCategory {
    let lower = prompt.to_ascii_lowercase();

    // Explore: file reading, listing, searching, simple lookups
    let explore_patterns = [
        "read ", "show ", "list ", "find ", "search ", "look at ",
        "what is", "where is", "how many", "count ",
        "git status", "git log", "git diff", "git show",
        "ls ", "cat ", "grep ", "open ", "check ",
        "show me", "what does", "print ",
    ];
    let explore_score = pattern_score(&lower, &explore_patterns);

    // Research: planning, analysis, architecture, investigation
    let research_patterns = [
        "plan", "design", "architect", "analyze", "analys",
        "investigate", "explore how", "research", "compare",
        "review", "audit", "evaluate", "assess", "explain how",
        "why does", "why is", "how does", "how should",
        "what approach", "strategy", "trade-off", "tradeoff",
        "pros and cons", "options for",
    ];
    let research_score = pattern_score(&lower, &research_patterns);

    // Code: implementation, refactoring, bug fixing, coding
    let code_patterns = [
        "implement", "refactor", "fix ", "bug ", "add ", "create ",
        "build ", "write code", "write a function", "write a method",
        "modify ", "change ", "update ", "patch ", "rewrite ",
        "optimize", "debug", "test ", "write test", "add test",
        "migrate", "port ", "convert ", "extract ", "inline ",
        "rename ", "move ", "split ", "merge ", "hook ",
        "endpoint", "handler", "middleware", "component",
        "function", "method", "class ", "struct ", "enum ",
        "api ", "route ", "query ", "mutation",
    ];
    let code_score = pattern_score(&lower, &code_patterns);

    // Write: documentation, messages, descriptions, explanations
    let write_patterns = [
        "write ", "document", "describe", "explain ", "summarize",
        "draft ", "compose", "generate ", "changelog", "readme",
        "commit message", "pr description", "comment ",
        "docstring", "jsdoc", "rustdoc", "annotation",
        "translate", "reword", "rephrase",
    ];
    let write_score = pattern_score(&lower, &write_patterns);

    // Pick the highest-scoring category, with code as default tiebreaker
    let scores = [
        (ActionCategory::Explore, explore_score),
        (ActionCategory::Research, research_score),
        (ActionCategory::Code, code_score),
        (ActionCategory::Write, write_score),
    ];

    scores
        .iter()
        .max_by_key(|(_, score)| *score)
        .filter(|(_, score)| *score > 0)
        .map(|(cat, _)| *cat)
        .unwrap_or(ActionCategory::Code) // Default to code for ambiguous prompts
}

/// Count how many patterns match in the text.
fn pattern_score(text: &str, patterns: &[&str]) -> usize {
    patterns.iter().filter(|p| text.contains(**p)).count()
}

/// Classify a model name into a tier.
fn classify_model(model: &str) -> ModelTier {
    let lower = model.to_ascii_lowercase();

    // Fast tier
    if lower.contains("haiku")
        || lower.contains("mini")
        || lower.contains("flash")
        || lower.contains("gpt-4o-mini")
        || lower.contains("grok-3-mini")
    {
        return ModelTier::Fast;
    }

    // Capable tier
    if lower.contains("opus")
        || lower.contains("gpt-4o") && !lower.contains("mini")
        || lower.contains("grok-3") && !lower.contains("mini")
        || lower.contains("pro")
    {
        return ModelTier::Capable;
    }

    // Everything else is balanced (sonnet, default, openanalyst-beta, etc.)
    ModelTier::Balanced
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_known_models() {
        assert_eq!(classify_model("claude-haiku-4-5"), ModelTier::Fast);
        assert_eq!(classify_model("claude-sonnet-4-6"), ModelTier::Balanced);
        assert_eq!(classify_model("claude-opus-4-6"), ModelTier::Capable);
        assert_eq!(classify_model("gpt-4o-mini"), ModelTier::Fast);
        assert_eq!(classify_model("grok-3-mini"), ModelTier::Fast);
        assert_eq!(classify_model("openanalyst-beta"), ModelTier::Balanced);
    }

    #[test]
    fn default_routing_table() {
        let table = RoutingTable::default();
        assert_eq!(table.explore.model_tier, ModelTier::Fast);
        assert_eq!(table.explore.effort, EffortLevel::Low);
        assert_eq!(table.code.model_tier, ModelTier::Capable);
        assert_eq!(table.code.effort, EffortLevel::High);
        assert_eq!(table.research.model_tier, ModelTier::Balanced);
        assert_eq!(table.write.effort, EffortLevel::Medium);
    }

    #[test]
    fn router_from_opus() {
        let router = ModelRouter::from_default_model("claude-opus-4-6");
        assert_eq!(router.model_for_agent(&AgentType::Explore), "claude-haiku-4-5");
        assert_eq!(router.model_for_agent(&AgentType::Plan), "claude-sonnet-4-6");
        assert_eq!(router.model_for_agent(&AgentType::Primary), "claude-opus-4-6");
    }

    #[test]
    fn prompt_classification() {
        assert_eq!(classify_prompt("read the file src/main.rs"), ActionCategory::Explore);
        assert_eq!(classify_prompt("list all files in src/"), ActionCategory::Explore);
        assert_eq!(classify_prompt("what is the current git status"), ActionCategory::Explore);

        assert_eq!(classify_prompt("plan the architecture for the new auth system"), ActionCategory::Research);
        assert_eq!(classify_prompt("analyze the trade-offs between these approaches"), ActionCategory::Research);

        assert_eq!(classify_prompt("refactor the auth module to use async"), ActionCategory::Code);
        assert_eq!(classify_prompt("implement a new caching layer"), ActionCategory::Code);
        assert_eq!(classify_prompt("fix the bug in the login handler"), ActionCategory::Code);
        assert_eq!(classify_prompt("add a test for the router"), ActionCategory::Code);

        assert_eq!(classify_prompt("write documentation for the API"), ActionCategory::Write);
        assert_eq!(classify_prompt("draft a PR description"), ActionCategory::Write);
        assert_eq!(classify_prompt("generate a changelog"), ActionCategory::Write);
    }

    #[test]
    fn route_prompt_full_stack() {
        let router = ModelRouter::from_default_model("claude-opus-4-6");

        let route = router.route_prompt("read src/main.rs");
        assert_eq!(route.category, ActionCategory::Explore);
        assert_eq!(route.model, "claude-haiku-4-5");
        assert_eq!(route.effort_budget, 1_024);

        let route = router.route_prompt("implement the new auth handler");
        assert_eq!(route.category, ActionCategory::Code);
        assert_eq!(route.model, "claude-opus-4-6");
        assert_eq!(route.effort_budget, 32_000);
    }

    #[test]
    fn set_effort_per_category() {
        let mut router = ModelRouter::from_default_model("claude-opus-4-6");
        router.table.set_effort(ActionCategory::Explore, EffortLevel::High);
        let route = router.route_prompt("read the file");
        assert_eq!(route.effort_budget, 32_000);
    }

    #[test]
    fn set_effort_global() {
        let mut router = ModelRouter::from_default_model("claude-opus-4-6");
        router.table.set_effort_all(EffortLevel::Max);
        for cat in ActionCategory::ALL {
            assert_eq!(router.table.get(cat).effort, EffortLevel::Max);
        }
    }

    #[test]
    fn set_tier_per_category() {
        let mut router = ModelRouter::from_default_model("claude-opus-4-6");
        router.table.set_tier(ActionCategory::Write, ModelTier::Capable);
        let route = router.route_prompt("write documentation for the API");
        assert_eq!(route.model, "claude-opus-4-6");
    }

    #[test]
    fn ambiguous_prompt_defaults_to_code() {
        assert_eq!(classify_prompt("hello world"), ActionCategory::Code);
        assert_eq!(classify_prompt("do the thing"), ActionCategory::Code);
    }

    #[test]
    fn sub_agent_routing() {
        let router = ModelRouter::from_default_model("claude-opus-4-6");

        let route = router.route_agent_task(&AgentType::Explore, "find all rust files");
        assert_eq!(route.category, ActionCategory::Explore);
        assert_eq!(route.model, "claude-haiku-4-5");

        let route = router.route_agent_task(&AgentType::Plan, "design the new API");
        assert_eq!(route.category, ActionCategory::Research);
        assert_eq!(route.model, "claude-sonnet-4-6");
    }
}
