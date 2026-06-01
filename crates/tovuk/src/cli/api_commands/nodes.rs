use super::super::{
    args::CliOptions,
    errors::{Result, agent_error},
    project::encode_component,
};
use super::generic::{print_authenticated, print_authenticated_mutation};
use reqwest::Method;
use serde_json::{Value, json};

pub(crate) fn nodes_command(cli: &CliOptions) -> Result<()> {
    match cli.args.first().map_or("list", String::as_str) {
        "list" => print_authenticated(cli, "/v1/operator/nodes"),
        "drain" => set_node_drain(cli, true),
        "enable" => set_node_drain(cli, false),
        _ => Err(agent_error(
            "unknown_command",
            "Unknown nodes command.",
            "Use `tovuk nodes list --json`, `tovuk nodes drain <node_id> --json`, or `tovuk nodes enable <node_id> --json` with an operator token.",
            cli.output.json,
        )),
    }
}

fn set_node_drain(cli: &CliOptions, draining: bool) -> Result<()> {
    let node_id = cli
        .args
        .get(1)
        .cloned()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            agent_error(
                "node_id_required",
                "Node id is required.",
                "Pass a node id returned by `tovuk nodes list --json`.",
                cli.output.json,
            )
        })?;
    print_authenticated_mutation(
        cli,
        Method::POST,
        &node_drain_route(&node_id),
        Some(node_drain_body(draining)),
    )
}

fn node_drain_route(node_id: &str) -> String {
    format!("/v1/operator/nodes/{}/drain", encode_component(node_id))
}

fn node_drain_body(draining: bool) -> Value {
    json!({ "draining": draining })
}

#[cfg(test)]
mod tests {
    use super::{node_drain_body, node_drain_route, nodes_command};
    use crate::cli::args::CliOptions;

    #[test]
    fn nodes_unknown_command_is_agent_readable() {
        let cli = CliOptions {
            command: "nodes".to_owned(),
            args: vec!["unknown".to_owned()],
            ..CliOptions::default()
        };

        let error_message = nodes_command(&cli).err().map(|error| error.to_string());
        assert_eq!(error_message.as_deref(), Some("Unknown nodes command."));
    }

    #[test]
    fn node_drain_requests_target_operator_route() {
        assert_eq!(
            node_drain_route("tovuk-riesling"),
            "/v1/operator/nodes/tovuk-riesling/drain"
        );
        assert_eq!(
            node_drain_body(true),
            serde_json::json!({ "draining": true })
        );
        assert_eq!(
            node_drain_body(false),
            serde_json::json!({ "draining": false })
        );
    }
}
