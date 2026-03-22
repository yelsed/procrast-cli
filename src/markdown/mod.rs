use crate::api::models::Idea;

pub fn idea_to_markdown(idea: &Idea) -> String {
    let mut md = String::new();

    // Title
    let title = idea
        .summary_title
        .as_deref()
        .unwrap_or("Untitled Idea");
    md.push_str(&format!("# {}\n\n", title));

    // Original idea
    md.push_str("## Original Idea\n\n");
    md.push_str(&idea.content);
    md.push_str("\n\n");

    // Refined version (if available)
    if let Some(ref refined) = idea.refined_content {
        if !refined.is_empty() {
            md.push_str("## Refined Version\n\n");
            md.push_str(refined);
            md.push_str("\n\n");
        }
    }

    // Summary (if available)
    if let Some(ref summary) = idea.refinement_summary {
        if !summary.is_empty() {
            md.push_str("## Summary\n\n");
            md.push_str(summary);
            md.push_str("\n\n");
        }
    }

    // Action steps (if available)
    if let Some(ref steps) = idea.action_steps {
        if !steps.is_empty() {
            md.push_str("## Action Steps\n\n");
            for step in steps {
                md.push_str(&format!("- {}\n", step));
            }
            md.push('\n');
        }
    }

    // Metadata
    md.push_str("## Metadata\n\n");
    md.push_str(&format!(
        "- Priority: {}\n",
        idea.priority.as_deref().unwrap_or("None")
    ));
    md.push_str(&format!(
        "- Due: {}\n",
        idea.due_date.as_deref().unwrap_or("None")
    ));
    md.push_str(&format!("- Created: {}\n", idea.created_at));
    md.push_str(&format!(
        "- Status: {}\n",
        idea.refinement_status.as_deref().unwrap_or("Not refined")
    ));
    md.push_str(&format!("- UUID: {}\n", idea.uuid));

    md
}
