//! Gjøremål fra Linear via GraphQL. Personlig API-nøkkel i LINEAR_API_KEY
//! (sendes rått i Authorization-headeren, uten «Bearer»). Henter åpne issues
//! på tvers av teams; ferdige/kansellerte filtreres bort i spørringen.

use anyhow::{Context, Result};
use schema::TodoItem;
use serde::Deserialize;
use serde_json::json;

const QUERY: &str = r#"
query DashboardTodos($filter: IssueFilter) {
  issues(filter: $filter, first: 30, orderBy: updatedAt) {
    nodes {
      title
      priority
      dueDate
      state { name type }
      project { name }
    }
  }
}"#;

#[derive(Deserialize)]
struct Resp {
    data: Option<Data>,
    errors: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct Data {
    issues: Issues,
}

#[derive(Deserialize)]
struct Issues {
    nodes: Vec<Node>,
}

#[derive(Deserialize)]
struct Node {
    title: String,
    /// 0 = ingen, 1 = urgent, 2 = high, 3 = medium, 4 = low
    priority: f64,
    #[serde(rename = "dueDate")]
    due_date: Option<String>,
    state: State,
    project: Option<Project>,
}

#[derive(Deserialize)]
struct State {
    name: String,
    #[serde(rename = "type")]
    state_type: String,
}

#[derive(Deserialize)]
struct Project {
    name: String,
}

pub async fn fetch(client: &reqwest::Client) -> Result<Vec<TodoItem>> {
    let Some(key) = crate::env_nonempty("LINEAR_API_KEY") else {
        return Ok(vec![]);
    };

    // LINEAR_PROJECT begrenser tavlen til ett prosjekt (felles-tavle med
    // reisepartner); uten settes hele workspacets åpne saker opp
    let mut filter = json!({
        "state": { "type": { "in": ["triage", "backlog", "unstarted", "started"] } }
    });
    if let Some(project) = crate::env_nonempty("LINEAR_PROJECT") {
        filter["project"] = json!({ "name": { "eq": project } });
    }

    let resp: Resp = client
        .post("https://api.linear.app/graphql")
        .header("Authorization", key)
        .json(&json!({ "query": QUERY, "variables": { "filter": filter } }))
        .send()
        .await
        .context("POST linear graphql")?
        .error_for_status()?
        .json()
        .await
        .context("parse linear-svar")?;

    if let Some(errors) = resp.errors {
        anyhow::bail!("linear graphql-feil: {errors}");
    }
    let mut nodes = resp
        .data
        .context("tomt linear-svar")?
        .issues
        .nodes;

    // Pågående først, deretter prioritet (urgent=1 er viktigst, 0=ingen sist),
    // så nærmeste frist
    let prio_rank = |p: f64| if p == 0.0 { 5.0 } else { p };
    nodes.sort_by(|a, b| {
        let started = |n: &Node| (n.state.state_type != "started") as u8;
        started(a)
            .cmp(&started(b))
            .then(prio_rank(a.priority).total_cmp(&prio_rank(b.priority)))
            .then(a.due_date.cmp(&b.due_date))
    });

    Ok(nodes
        .into_iter()
        .map(|n| TodoItem {
            title: n.title,
            status: n.state.name,
            project: n.project.map(|p| p.name),
            due: n.due_date,
        })
        .collect())
}
