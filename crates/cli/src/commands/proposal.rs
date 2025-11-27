//! Governance proposal commands

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::commands::CommandContext;
use crate::interactive::{confirm_default_yes, prompt_input, spinner};
use crate::output::{colors, TableFormatter};

#[derive(Debug, Serialize, Deserialize)]
pub struct Proposal {
    pub id: String,
    pub title: String,
    pub description: String,
    pub proposal_type: String,
    pub status: String,
    pub proposer_id: String,
    pub created_at: String,
    pub voting_ends_at: Option<String>,
    pub votes_for: u32,
    pub votes_against: u32,
    pub votes_abstain: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProposalList {
    pub proposals: Vec<Proposal>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct CreateProposalRequest {
    pub title: String,
    pub description: String,
    pub proposal_type: String,
    pub content: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct VoteRequest {
    pub vote: String,
}

#[derive(Debug, Serialize)]
pub struct CommentRequest {
    pub message: String,
}

/// List governance proposals
pub async fn list(ctx: &CommandContext, status: Option<String>) -> Result<()> {
    let sp = spinner("Fetching proposals...");

    let path = if let Some(s) = status {
        format!("/api/v1/proposals?status={}", s)
    } else {
        "/api/v1/proposals".to_string()
    };

    let list: ProposalList = ctx.client.get(&path).await?;

    sp.finish_and_clear();

    if list.proposals.is_empty() {
        println!("{}", colors::warning("No proposals found."));
        return Ok(());
    }

    let headers = vec!["ID", "Title", "Type", "Status", "For", "Against", "Abstain"];
    let rows: Vec<Vec<String>> = list
        .proposals
        .iter()
        .map(|p| {
            vec![
                p.id.clone(),
                p.title.clone(),
                p.proposal_type.clone(),
                p.status.clone(),
                p.votes_for.to_string(),
                p.votes_against.to_string(),
                p.votes_abstain.to_string(),
            ]
        })
        .collect();

    let table = TableFormatter::simple(headers, rows)?;
    println!("{}", table);
    println!("{} proposals found", colors::dim(&list.total.to_string()));

    Ok(())
}

/// Show proposal details
pub async fn show(ctx: &CommandContext, proposal_id: String) -> Result<()> {
    let sp = spinner("Fetching proposal details...");

    let proposal: Proposal = ctx
        .client
        .get(&format!("/api/v1/proposals/{}", proposal_id))
        .await?;

    sp.finish_and_clear();

    println!("{}", colors::bold("Proposal Details"));
    println!();

    let items = vec![
        ("ID", proposal.id),
        ("Title", proposal.title),
        ("Type", proposal.proposal_type),
        ("Status", proposal.status),
        ("Proposer ID", proposal.proposer_id),
        ("Created At", proposal.created_at),
        (
            "Voting Ends",
            proposal.voting_ends_at.unwrap_or_else(|| "-".to_string()),
        ),
        ("Votes For", proposal.votes_for.to_string()),
        ("Votes Against", proposal.votes_against.to_string()),
        ("Votes Abstain", proposal.votes_abstain.to_string()),
    ];

    let table = TableFormatter::key_value(items)?;
    println!("{}", table);

    println!();
    println!("{}", colors::bold("Description:"));
    println!("{}", proposal.description);

    Ok(())
}

/// Create a new proposal
pub async fn create(
    ctx: &CommandContext,
    proposal_type: String,
    file_path: Option<String>,
) -> Result<()> {
    ctx.require_auth()?;

    let title = prompt_input("Proposal title")?;
    let description = prompt_input("Proposal description")?;

    let content = if let Some(path) = file_path {
        let path = Path::new(&path);
        if !path.exists() {
            anyhow::bail!("File not found: {}", path.display());
        }

        let file_content = fs::read_to_string(path)
            .context("Failed to read proposal content file")?;

        if path.extension().and_then(|s| s.to_str()) == Some("yaml")
            || path.extension().and_then(|s| s.to_str()) == Some("yml")
        {
            let yaml: serde_yaml::Value = serde_yaml::from_str(&file_content)
                .context("Failed to parse YAML")?;
            serde_json::to_value(yaml)?
        } else {
            serde_json::from_str(&file_content).context("Failed to parse JSON")?
        }
    } else {
        serde_json::json!({})
    };

    println!("{}", colors::bold("Creating proposal:"));
    println!("  Title: {}", title);
    println!("  Type:  {}", proposal_type);
    println!();

    let confirmed = confirm_default_yes("Create this proposal?")?;
    if !confirmed {
        println!("Cancelled.");
        return Ok(());
    }

    let sp = spinner("Creating proposal...");

    let request = CreateProposalRequest {
        title,
        description,
        proposal_type,
        content,
    };

    let proposal: Proposal = ctx.client.post("/api/v1/proposals", &request).await?;

    sp.finish_and_clear();

    println!("{}", colors::success("Proposal created successfully!"));
    println!("ID: {}", proposal.id);

    Ok(())
}

/// Vote on a proposal
pub async fn vote(ctx: &CommandContext, proposal_id: String, vote: String) -> Result<()> {
    ctx.require_auth()?;

    let vote_type = match vote.to_lowercase().as_str() {
        "approve" | "for" | "yes" => "approve",
        "reject" | "against" | "no" => "reject",
        "abstain" => "abstain",
        _ => anyhow::bail!("Invalid vote type. Use: approve, reject, or abstain"),
    };

    println!("{}", colors::bold("Voting on proposal:"));
    println!("  Proposal ID: {}", proposal_id);
    println!("  Vote:        {}", vote_type);
    println!();

    let confirmed = confirm_default_yes("Submit this vote?")?;
    if !confirmed {
        println!("Cancelled.");
        return Ok(());
    }

    let sp = spinner("Submitting vote...");

    let request = VoteRequest {
        vote: vote_type.to_string(),
    };

    let _: serde_json::Value = ctx
        .client
        .post(&format!("/api/v1/proposals/{}/vote", proposal_id), &request)
        .await?;

    sp.finish_and_clear();

    println!("{}", colors::success("Vote submitted successfully!"));

    Ok(())
}

/// Comment on a proposal
pub async fn comment(
    ctx: &CommandContext,
    proposal_id: String,
    message: Option<String>,
) -> Result<()> {
    ctx.require_auth()?;

    let comment_text = if let Some(msg) = message {
        msg
    } else {
        prompt_input("Comment")?
    };

    let sp = spinner("Posting comment...");

    let request = CommentRequest {
        message: comment_text,
    };

    let _: serde_json::Value = ctx
        .client
        .post(
            &format!("/api/v1/proposals/{}/comments", proposal_id),
            &request,
        )
        .await?;

    sp.finish_and_clear();

    println!("{}", colors::success("Comment posted successfully!"));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proposal_serialization() {
        let proposal = Proposal {
            id: "prop-123".to_string(),
            title: "Test Proposal".to_string(),
            description: "A test proposal".to_string(),
            proposal_type: "new-benchmark".to_string(),
            status: "active".to_string(),
            proposer_id: "user-456".to_string(),
            created_at: "2024-01-01".to_string(),
            voting_ends_at: None,
            votes_for: 10,
            votes_against: 5,
            votes_abstain: 2,
        };

        let json = serde_json::to_string(&proposal).unwrap();
        assert!(json.contains("prop-123"));
    }
}
