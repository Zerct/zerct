use super::super::{
    args::CliOptions,
    errors::{Result, agent_error},
    project::encode_component,
};
use super::{
    common::{app_route, page_query},
    generic::print_authenticated,
};

pub(crate) fn service_command(cli: &CliOptions) -> Result<()> {
    match cli.args.first().map_or("list", String::as_str) {
        "list" => print_authenticated(cli, "/v1/services"),
        "show" => {
            let service = cli.args.get(1).cloned().filter(|value| !value.is_empty());
            let route = if let Some(service) = service {
                format!("/v1/services/{}/overview", encode_component(&service))
            } else {
                app_route(cli, "overview")?
            };
            print_authenticated(cli, &route)
        }
        _ => Err(agent_error(
            "unknown_command",
            "Unknown service command.",
            "Use `tovuk service list --json` or `tovuk service show <service> --json`.",
            cli.output.json,
        )),
    }
}

pub(crate) fn deploys_command(cli: &CliOptions) -> Result<()> {
    let route = if cli.app.is_empty() {
        format!("/v1/deploys{}", page_query(cli))
    } else {
        format!(
            "/v1/services/{}/deploys{}",
            encode_component(&cli.app),
            page_query(cli)
        )
    };
    print_authenticated(cli, &route)
}

pub(crate) fn builds_command(cli: &CliOptions) -> Result<()> {
    let route = if cli.app.is_empty() {
        format!("/v1/builds{}", page_query(cli))
    } else {
        format!(
            "/v1/services/{}/builds{}",
            encode_component(&cli.app),
            page_query(cli)
        )
    };
    print_authenticated(cli, &route)
}
