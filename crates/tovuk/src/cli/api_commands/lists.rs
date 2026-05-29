use super::super::{args::CliOptions, errors::Result, project::encode_component};
use super::{common::page_query, generic::print_authenticated};

pub(crate) fn deploys_command(cli: &CliOptions) -> Result<()> {
    let route = if cli.app.is_empty() {
        format!("/v1/deploys{}", page_query(cli))
    } else {
        format!(
            "/v1/apps/{}/deploys{}",
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
            "/v1/apps/{}/builds{}",
            encode_component(&cli.app),
            page_query(cli)
        )
    };
    print_authenticated(cli, &route)
}
