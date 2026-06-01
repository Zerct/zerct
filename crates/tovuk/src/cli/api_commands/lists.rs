use super::super::{
    args::CliOptions,
    errors::{Result, agent_error},
    project::encode_component,
};
use super::{
    common::{page_query, service_route},
    generic::{print_authenticated, print_authenticated_mutation},
};
use reqwest::Method;

pub(crate) fn service_command(cli: &CliOptions) -> Result<()> {
    match cli.args.first().map_or("list", String::as_str) {
        "list" => print_authenticated(cli, "/v1/services"),
        "show" => service_get(cli, 1, "overview"),
        "status" => service_get(cli, 1, "status"),
        "resources" => service_get(cli, 1, "platform"),
        "deploys" => service_paged_get(cli, 1, "deploys"),
        "builds" => service_paged_get(cli, 1, "builds"),
        "delete" | "del" | "rm" => {
            let service = cli.args.get(1).cloned().filter(|value| !value.is_empty());
            let route = if let Some(service) = service {
                format!("/v1/services/{}", encode_component(&service))
            } else {
                service_route(cli, "")?
            };
            print_authenticated_mutation(cli, Method::DELETE, &route, None)
        }
        _ => Err(agent_error(
            "unknown_command",
            "Unknown service command.",
            "Use `tovuk service list --json`, `tovuk service show <service> --json`, `tovuk service status <service> --json`, `tovuk service resources <service> --json`, `tovuk service deploys <service> --json`, `tovuk service builds <service> --json`, or `tovuk service delete <service> --json`.",
            cli.output.json,
        )),
    }
}

fn service_get(cli: &CliOptions, arg_index: usize, suffix: &str) -> Result<()> {
    print_authenticated(cli, &service_route_from_arg(cli, arg_index, suffix)?)
}

fn service_paged_get(cli: &CliOptions, arg_index: usize, suffix: &str) -> Result<()> {
    let route = format!(
        "{}{}",
        service_route_from_arg(cli, arg_index, suffix)?,
        page_query(cli)
    );
    print_authenticated(cli, &route)
}

fn service_route_from_arg(cli: &CliOptions, arg_index: usize, suffix: &str) -> Result<String> {
    let service = cli
        .args
        .get(arg_index)
        .cloned()
        .filter(|value| !value.is_empty());
    if let Some(service) = service {
        let suffix = suffix.trim_matches('/');
        if suffix.is_empty() {
            return Ok(format!("/v1/services/{}", encode_component(&service)));
        }
        return Ok(format!(
            "/v1/services/{}/{}",
            encode_component(&service),
            suffix
        ));
    }
    service_route(cli, suffix)
}
