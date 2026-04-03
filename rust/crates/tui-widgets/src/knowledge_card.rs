//! Knowledge base result card — tabbed, collapsible, with abstracted source labels.
//!
//! Renders as a bordered card with:
//! - Intent tag + latency in the title
//! - Tab bar for sub-questions (Left/Right to cycle)
//! - Result list per tab with category labels and confidence scores
//! - Synthesized answer section
//! - Cache indicator when served from local cache

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Padding, Paragraph, Widget, Wrap};

/// A single result entry in a knowledge card tab.
#[derive(Debug, Clone)]
pub struct KbResultEntry {
    /// Abstracted category label — e.g., "Ads Strategy", never raw course name.
    pub category_label: String,
    /// Short snippet of the chunk text.
    pub snippet: String,
    /// Relevance score (0.0 - 1.0).
    pub score: f64,
    /// Citation label — e.g., "[Ads Strategy #1]".
    pub citation_label: String,
    /// Whether this result was found via Neo4j graph traversal.
    pub graph_expanded: bool,
}

/// One tab in the knowledge card — corresponds to one sub-question.
#[derive(Debug, Clone)]
pub struct KnowledgeTab {
    /// Sub-question text (tab title is a truncated version).
    pub sub_question: String,
    /// Intent tag for this sub-question.
    pub intent: String,
    /// Results for this sub-question.
    pub results: Vec<KbResultEntry>,
}

/// A knowledge base result card with tabbed sub-question results.
#[derive(Debug, Clone)]
pub struct KnowledgeCard {
    /// The original query.
    pub query: String,
    /// Classified intent tag.
    pub intent: String,
    /// Overall latency in milliseconds.
    pub latency_ms: u64,
    /// Tabs — one per sub-question.
    pub tabs: Vec<KnowledgeTab>,
    /// Active tab index.
    pub active_tab: usize,
    /// Whether the card is expanded.
    pub expanded: bool,
    /// Synthesized answer text.
    pub answer: Option<String>,
    /// Whether this result came from local cache.
    pub from_cache: bool,
    /// Whether feedback has been submitted.
    pub feedback_submitted: bool,
    /// Query ID for feedback.
    pub query_id: i64,
}

impl KnowledgeCard {
    /// Toggle the expanded/collapsed state.
    pub fn toggle_expand(&mut self) {
        self.expanded = !self.expanded;
    }

    /// Cycle to the next tab.
    pub fn next_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_tab = (self.active_tab + 1) % self.tabs.len();
        }
    }

    /// Cycle to the previous tab.
    pub fn prev_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_tab = if self.active_tab == 0 {
                self.tabs.len() - 1
            } else {
                self.active_tab - 1
            };
        }
    }
}

impl Widget for KnowledgeCard {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 3 || area.width < 10 {
            return;
        }

        let border_color = if self.expanded {
            Color::Cyan
        } else {
            Color::Indexed(245)
        };

        // Title: ─ KB ── Strategic ── 2.3s ── ⚡cached ─
        let latency_str = if self.latency_ms < 1000 {
            format!("{}ms", self.latency_ms)
        } else {
            format!("{:.1}s", self.latency_ms as f64 / 1000.0)
        };

        let mut title_spans = vec![
            Span::styled("─ ", Style::default().fg(border_color)),
            Span::styled("✦ ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(
                "KB",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" ── ", Style::default().fg(border_color)),
            Span::styled(
                &self.intent,
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(
                format!(" ── {latency_str} "),
                Style::default().fg(border_color),
            ),
        ];

        if self.from_cache {
            title_spans.push(Span::styled(
                "── ⚡cached ",
                Style::default().fg(Color::Green),
            ));
        }

        let expand_hint = if self.expanded { "▾" } else { "▸" };
        title_spans.push(Span::styled(
            format!("{expand_hint} "),
            Style::default().fg(border_color),
        ));

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color))
            .title(Line::from(title_spans))
            .padding(Padding::horizontal(1));

        let inner = block.inner(area);
        block.render(area, buf);

        if !self.expanded {
            // Collapsed: single-line summary
            let result_count: usize = self.tabs.iter().map(|t| t.results.len()).sum();
            let line = Line::from(vec![
                Span::styled(
                    format!("{result_count} results"),
                    Style::default().fg(Color::Indexed(252)),
                ),
                Span::styled(
                    format!(" · {} sub-queries", self.tabs.len()),
                    Style::default().fg(Color::Indexed(240)),
                ),
            ]);
            Paragraph::new(vec![line]).render(inner, buf);
            return;
        }

        let mut lines: Vec<Line<'_>> = Vec::new();
        let max_w = inner.width as usize;

        // Tab bar
        if self.tabs.len() > 1 {
            let mut tab_spans = Vec::new();
            for (i, tab) in self.tabs.iter().enumerate() {
                let label = truncate_kb(&tab.sub_question, 20);
                if i == self.active_tab {
                    tab_spans.push(Span::styled(
                        format!(" [{label}] "),
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ));
                } else {
                    tab_spans.push(Span::styled(
                        format!("  {label}  "),
                        Style::default().fg(Color::Indexed(245)),
                    ));
                }
            }
            lines.push(Line::from(tab_spans));
            lines.push(Line::from(""));
        }

        // Active tab results
        if let Some(tab) = self.tabs.get(self.active_tab) {
            for entry in tab.results.iter().take(8) {
                let graph_tag = if entry.graph_expanded { " ⇔" } else { "" };
                let score_pct = (entry.score * 100.0) as u32;
                lines.push(Line::from(vec![
                    Span::styled(
                        &entry.citation_label,
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!(" ({score_pct}%{graph_tag}) "),
                        Style::default().fg(Color::Indexed(240)),
                    ),
                ]));
                lines.push(Line::from(Span::styled(
                    truncate_kb(&entry.snippet, max_w),
                    Style::default().fg(Color::Indexed(252)),
                )));
                lines.push(Line::from(""));
            }
        }

        // Synthesized answer
        if let Some(ref answer) = self.answer {
            lines.push(Line::from(Span::styled(
                "── Answer ──",
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            )));
            for line in answer.lines().take(30) {
                lines.push(Line::from(Span::raw(truncate_kb(line, max_w))));
            }
        }

        Paragraph::new(lines).wrap(Wrap { trim: false }).render(inner, buf);
    }
}

/// Calculate the height needed to render a knowledge card.
#[must_use]
pub fn knowledge_card_height(card: &KnowledgeCard, _width: u16) -> u16 {
    if !card.expanded {
        return 3; // border + summary + border
    }
    let mut h: u16 = 2; // borders
    if card.tabs.len() > 1 {
        h += 2; // tab bar + blank line
    }
    if let Some(tab) = card.tabs.get(card.active_tab) {
        h += (tab.results.len().min(8) * 3) as u16; // citation + snippet + blank per result
    }
    if card.answer.is_some() {
        h += 2; // header + at least one line
    }
    h.max(3)
}

/// UTF-8 safe truncation.
fn truncate_kb(s: &str, max: usize) -> String {
    let count = s.chars().count();
    if count <= max {
        s.to_string()
    } else if max > 3 {
        let t: String = s.chars().take(max - 3).collect();
        format!("{t}...")
    } else {
        s.chars().take(max).collect()
    }
}
