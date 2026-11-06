//! Contextual Usage & Cost Insights Engine (Phase 10).
//!
//! Provides deterministic token aggregation, model pricing evaluation, and
//! transparent provenance disclosures for Agent sessions and Workspaces.
//! Conforms strictly to ROADMAP:
//! "Usage carries provenance/time/unit; estimated or unknown cost is labeled.
//!  No project/session upload by default; user can remove retained references."

use crate::domain::AgentId;
use crate::store::{SessionRecord, WorkspaceRecord};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Official reference pricing (cents per 1,000,000 tokens) as of 2026.
struct PricingEntry {
    input_cents_per_million: u64,
    output_cents_per_million: u64,
    display_name: &'static str,
}

const PRICING_TABLE: &[(&str, PricingEntry)] = &[
    (
        "claude-3-5-sonnet",
        PricingEntry {
            input_cents_per_million: 300,   // $3.00
            output_cents_per_million: 1500, // $15.00
            display_name: "Claude 3.5 Sonnet",
        },
    ),
    (
        "claude-3-5-haiku",
        PricingEntry {
            input_cents_per_million: 100,  // $1.00
            output_cents_per_million: 500, // $5.00
            display_name: "Claude 3.5 Haiku",
        },
    ),
    (
        "claude-3-opus",
        PricingEntry {
            input_cents_per_million: 1500,  // $15.00
            output_cents_per_million: 7500, // $75.00
            display_name: "Claude 3 Opus",
        },
    ),
    (
        "gpt-4o",
        PricingEntry {
            input_cents_per_million: 250,   // $2.50
            output_cents_per_million: 1000, // $10.00
            display_name: "GPT-4o",
        },
    ),
    (
        "gpt-4o-mini",
        PricingEntry {
            input_cents_per_million: 15,  // $0.15
            output_cents_per_million: 60, // $0.60
            display_name: "GPT-4o mini",
        },
    ),
    (
        "o1-preview",
        PricingEntry {
            input_cents_per_million: 1500,  // $15.00
            output_cents_per_million: 6000, // $60.00
            display_name: "o1-preview",
        },
    ),
    (
        "o1-mini",
        PricingEntry {
            input_cents_per_million: 300,   // $3.00
            output_cents_per_million: 1200, // $12.00
            display_name: "o1-mini",
        },
    ),
    (
        "gemini-1.5-pro",
        PricingEntry {
            input_cents_per_million: 125,  // $1.25
            output_cents_per_million: 500, // $5.00
            display_name: "Gemini 1.5 Pro",
        },
    ),
    (
        "gemini-1.5-flash",
        PricingEntry {
            input_cents_per_million: 8,   // $0.075 rounded
            output_cents_per_million: 30, // $0.30
            display_name: "Gemini 1.5 Flash",
        },
    ),
    (
        "claude-opus",
        PricingEntry {
            input_cents_per_million: 1500,  // $15.00
            output_cents_per_million: 7500, // $75.00
            display_name: "Claude Opus",
        },
    ),
    (
        "claude-sonnet",
        PricingEntry {
            input_cents_per_million: 300,   // $3.00
            output_cents_per_million: 1500, // $15.00
            display_name: "Claude Sonnet",
        },
    ),
    (
        "opus",
        PricingEntry {
            input_cents_per_million: 1500,
            output_cents_per_million: 7500,
            display_name: "Claude Opus",
        },
    ),
    (
        "sonnet",
        PricingEntry {
            input_cents_per_million: 300,
            output_cents_per_million: 1500,
            display_name: "Claude Sonnet",
        },
    ),
    (
        "gpt-6",
        PricingEntry {
            input_cents_per_million: 250,
            output_cents_per_million: 1000,
            display_name: "GPT-6 Codex",
        },
    ),
    (
        "gpt-5",
        PricingEntry {
            input_cents_per_million: 200,
            output_cents_per_million: 800,
            display_name: "GPT-5",
        },
    ),
    (
        "o3",
        PricingEntry {
            input_cents_per_million: 1000,
            output_cents_per_million: 4000,
            display_name: "OpenAI o3",
        },
    ),
    (
        "gemini-2.5-pro",
        PricingEntry {
            input_cents_per_million: 125,
            output_cents_per_million: 500,
            display_name: "Gemini 2.5 Pro",
        },
    ),
    (
        "gemini-2.5-flash",
        PricingEntry {
            input_cents_per_million: 10,
            output_cents_per_million: 40,
            display_name: "Gemini 2.5 Flash",
        },
    ),
];

/// Calculates estimated cost in cents based on known model rates.
/// Returns (cents, provenance_description, is_known).
pub fn calculate_cost_cents(
    model: Option<&str>,
    input_tokens: u64,
    output_tokens: u64,
) -> (u64, String, bool) {
    let model_id = model.unwrap_or("").trim().to_lowercase();
    if model_id.is_empty() {
        return (0, "pricing.unknown".to_string(), false);
    }

    for (key, entry) in PRICING_TABLE {
        if model_id.contains(key) {
            // Cents = (in_tok * in_rate + out_tok * out_rate) / 1,000,000
            let in_cost =
                (input_tokens as u128 * entry.input_cents_per_million as u128) / 1_000_000;
            let out_cost =
                (output_tokens as u128 * entry.output_cents_per_million as u128) / 1_000_000;
            let total_cents = (in_cost + out_cost) as u64;
            return (
                total_cents,
                format!("pricing.official:{}", entry.display_name),
                true,
            );
        }
    }

    (0, "pricing.unknown_or_free".to_string(), false)
}

pub fn format_usd(cents: u64) -> String {
    let dollars = cents / 100;
    let rem = cents % 100;
    format!("${}.{:02}", dollars, rem)
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentUsageBreakdown {
    pub agent_id: AgentId,
    pub session_count: usize,
    pub total_tokens: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub estimated_cost_usd: String,
    pub percentage_of_total: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModelUsageBreakdown {
    pub model_identifier: String,
    pub session_count: usize,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub estimated_cost_usd: String,
    pub is_estimated: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceUsageBreakdown {
    pub workspace_id: String,
    pub workspace_name: String,
    pub session_count: usize,
    pub total_tokens: u64,
    pub estimated_cost_usd: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UsageInsights {
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_tokens: u64,
    pub total_estimated_cost_usd: String,
    pub total_sessions_count: usize,
    pub by_agent: Vec<AgentUsageBreakdown>,
    pub by_model: Vec<ModelUsageBreakdown>,
    pub by_workspace: Vec<WorkspaceUsageBreakdown>,
    pub provenance_notes: Vec<String>,
}

pub fn compute_usage_insights(
    sessions: &[SessionRecord],
    workspaces: &[WorkspaceRecord],
) -> UsageInsights {
    let mut total_in = 0u64;
    let mut total_out = 0u64;
    let mut total_cost_cents = 0u64;

    struct AgentAcc {
        count: usize,
        in_tok: u64,
        out_tok: u64,
        cost_cents: u64,
    }
    let mut agent_map: HashMap<AgentId, AgentAcc> = HashMap::new();

    struct ModelAcc {
        count: usize,
        in_tok: u64,
        out_tok: u64,
        cost_cents: u64,
        is_known: bool,
    }
    let mut model_map: HashMap<String, ModelAcc> = HashMap::new();

    struct WsAcc {
        count: usize,
        in_tok: u64,
        out_tok: u64,
        cost_cents: u64,
    }
    let mut ws_map: HashMap<String, WsAcc> = HashMap::new();

    let ws_names: HashMap<String, String> = workspaces
        .iter()
        .map(|w| (w.id.clone(), w.name.clone()))
        .collect();

    for s in sessions {
        total_in += s.input_tokens;
        total_out += s.output_tokens;
        total_cost_cents += s.estimated_cost_cents;

        if let Some(aid) = AgentId::parse(&s.agent_id) {
            let acc = agent_map.entry(aid).or_insert(AgentAcc {
                count: 0,
                in_tok: 0,
                out_tok: 0,
                cost_cents: 0,
            });
            acc.count += 1;
            acc.in_tok += s.input_tokens;
            acc.out_tok += s.output_tokens;
            acc.cost_cents += s.estimated_cost_cents;
        }

        let model_name = s
            .model_identifier
            .as_deref()
            .unwrap_or("Default / Unknown")
            .to_string();
        let is_known = s.estimated_cost_cents > 0 || !model_name.contains("Unknown");
        let m_acc = model_map.entry(model_name).or_insert(ModelAcc {
            count: 0,
            in_tok: 0,
            out_tok: 0,
            cost_cents: 0,
            is_known,
        });
        m_acc.count += 1;
        m_acc.in_tok += s.input_tokens;
        m_acc.out_tok += s.output_tokens;
        m_acc.cost_cents += s.estimated_cost_cents;

        if let Some(ref wid) = s.workspace_id {
            let w_acc = ws_map.entry(wid.clone()).or_insert(WsAcc {
                count: 0,
                in_tok: 0,
                out_tok: 0,
                cost_cents: 0,
            });
            w_acc.count += 1;
            w_acc.in_tok += s.input_tokens;
            w_acc.out_tok += s.output_tokens;
            w_acc.cost_cents += s.estimated_cost_cents;
        }
    }

    let grand_total_tokens = total_in + total_out;

    let mut by_agent = Vec::new();
    for (aid, acc) in agent_map {
        let tot = acc.in_tok + acc.out_tok;
        let pct = if grand_total_tokens > 0 {
            (tot as f32 / grand_total_tokens as f32) * 100.0
        } else {
            0.0
        };
        by_agent.push(AgentUsageBreakdown {
            agent_id: aid,
            session_count: acc.count,
            total_tokens: tot,
            input_tokens: acc.in_tok,
            output_tokens: acc.out_tok,
            estimated_cost_usd: format_usd(acc.cost_cents),
            percentage_of_total: pct,
        });
    }
    by_agent.sort_by_key(|a| std::cmp::Reverse(a.total_tokens));

    let mut by_model = Vec::new();
    for (name, acc) in model_map {
        by_model.push(ModelUsageBreakdown {
            model_identifier: name,
            session_count: acc.count,
            input_tokens: acc.in_tok,
            output_tokens: acc.out_tok,
            total_tokens: acc.in_tok + acc.out_tok,
            estimated_cost_usd: format_usd(acc.cost_cents),
            is_estimated: acc.is_known,
        });
    }
    by_model.sort_by_key(|a| std::cmp::Reverse(a.total_tokens));

    let mut by_workspace = Vec::new();
    for (wid, acc) in ws_map {
        let name = ws_names
            .get(&wid)
            .cloned()
            .unwrap_or_else(|| "Unknown Workspace".to_string());
        by_workspace.push(WorkspaceUsageBreakdown {
            workspace_id: wid,
            workspace_name: name,
            session_count: acc.count,
            total_tokens: acc.in_tok + acc.out_tok,
            estimated_cost_usd: format_usd(acc.cost_cents),
        });
    }
    by_workspace.sort_by_key(|a| std::cmp::Reverse(a.total_tokens));

    let provenance_notes = vec![
        "usage.provenance.local_only".to_string(),
        "usage.provenance.official_rates".to_string(),
        "usage.provenance.non_billing_estimate".to_string(),
    ];

    UsageInsights {
        total_input_tokens: total_in,
        total_output_tokens: total_out,
        total_tokens: grand_total_tokens,
        total_estimated_cost_usd: format_usd(total_cost_cents),
        total_sessions_count: sessions.len(),
        by_agent,
        by_model,
        by_workspace,
        provenance_notes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pricing_calculation_matches_official_rates() {
        // 1M input + 1M output for Claude 3.5 Sonnet = $3.00 + $15.00 = $18.00 (1800 cents)
        let (cents, prov, known) =
            calculate_cost_cents(Some("claude-3-5-sonnet-20241022"), 1_000_000, 1_000_000);
        assert!(known);
        assert_eq!(cents, 1800);
        assert!(prov.contains("Claude 3.5 Sonnet"));

        // 10,000 input + 2,000 output for GPT-4o:
        // (10,000 * 250) / 1,000,000 = 2.5 -> 2 cents
        // (2,000 * 1000) / 1,000,000 = 2 cents
        // Total = 4 cents
        let (cents_gpt, _, known_gpt) = calculate_cost_cents(Some("gpt-4o"), 10_000, 2_000);
        assert!(known_gpt);
        assert_eq!(cents_gpt, 4);

        // Unknown model
        let (cents_unk, prov_unk, known_unk) =
            calculate_cost_cents(Some("llama-3-local"), 50_000, 10_000);
        assert!(!known_unk);
        assert_eq!(cents_unk, 0);
        assert_eq!(prov_unk, "pricing.unknown_or_free");
    }

    #[test]
    fn usage_insights_aggregation() {
        let sessions = vec![
            SessionRecord {
                id: "s1".into(),
                agent_id: "claude".into(),
                workspace_id: Some("w1".into()),
                external_session_id: "ext1".into(),
                title: "Refactor".into(),
                project_path: "/test/p1".into(),
                started_at: "2026-11-05T00:00:00Z".into(),
                last_active_at: "2026-11-05T00:10:00Z".into(),
                message_count: 5,
                input_tokens: 10_000,
                output_tokens: 2_000,
                estimated_cost_cents: 6,
                cost_provenance: "official".into(),
                model_identifier: Some("claude-3-5-sonnet".into()),
                created_at: "2026-11-05T00:00:00Z".into(),
            },
            SessionRecord {
                id: "s2".into(),
                agent_id: "codex".into(),
                workspace_id: Some("w1".into()),
                external_session_id: "ext2".into(),
                title: "Add tests".into(),
                project_path: "/test/p1".into(),
                started_at: "2026-11-05T01:00:00Z".into(),
                last_active_at: "2026-11-05T01:15:00Z".into(),
                message_count: 8,
                input_tokens: 20_000,
                output_tokens: 4_000,
                estimated_cost_cents: 9,
                cost_provenance: "official".into(),
                model_identifier: Some("gpt-4o".into()),
                created_at: "2026-11-05T01:00:00Z".into(),
            },
        ];

        let workspaces = vec![WorkspaceRecord {
            id: "w1".into(),
            name: "Project One".into(),
            path: "/test/p1".into(),
            primary_agent: "claude".into(),
            fallback_agent: None,
            model_id: None,
            profile_id: None,
            created_at: "2026-11-05T00:00:00Z".into(),
            updated_at: "2026-11-05T00:00:00Z".into(),
            last_opened_at: None,
        }];

        let insights = compute_usage_insights(&sessions, &workspaces);
        assert_eq!(insights.total_input_tokens, 30_000);
        assert_eq!(insights.total_output_tokens, 6_000);
        assert_eq!(insights.total_tokens, 36_000);
        assert_eq!(insights.total_estimated_cost_usd, "$0.15");
        assert_eq!(insights.total_sessions_count, 2);

        assert_eq!(insights.by_agent.len(), 2);
        assert_eq!(insights.by_workspace.len(), 1);
        assert_eq!(insights.by_workspace[0].workspace_name, "Project One");
        assert_eq!(insights.by_model.len(), 2);
    }
}
