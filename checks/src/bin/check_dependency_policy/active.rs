//! Active Cargo metadata pruning and serialization.

use alloc::collections::BTreeSet;

use serde_json::{from_slice, to_value, to_vec};

use tovuk_public_checks::{check_support::CheckResult, check_try};

use super::{
    ActiveMetadata, MetadataPackage, MetadataSnapshot, ResolveNode,
    graph::GraphContext,
    tree::{self, ActiveGraph, ActiveNode},
};

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0007] = [
    size_of_val(&load_active_metadata),
    size_of_val(&load_lock_wide_metadata),
    size_of_val(&parse_metadata),
    size_of_val(&prune_metadata),
    size_of_val(&rewrite_node),
    size_of_val(&serialize_metadata),
    size_of_val(&validate_pruned),
];

/// Load, prune, and serialize one target-specific Cargo metadata document.
///
/// # Errors
///
/// Returns an error when metadata or the active Cargo tree is inconsistent.
pub(super) fn load_active_metadata(
    context: &GraphContext,
    metadata_json: &[u8],
) -> CheckResult<ActiveMetadata> {
    let metadata = check_try!(parse_metadata(metadata_json));
    let graph = check_try!(tree::load_graph(context, &metadata));
    return serialize_metadata(check_try!(prune_metadata(metadata, &graph))).map_err(|error| {
        return format!("{} {}: {error}", context.manifest(), context.triple());
    });
}

/// Serialize metadata with every locked package promoted to a graph root.
///
/// # Errors
///
/// Returns an error when package identities or workspace metadata are invalid.
pub(super) fn load_lock_wide_metadata(metadata_json: &[u8]) -> CheckResult<Vec<u8>> {
    let mut metadata = check_try!(parse_metadata(metadata_json));
    let members = metadata
        .packages
        .iter()
        .map(|package| return package.identifier.clone())
        .collect::<BTreeSet<_>>();
    if members.len() != metadata.packages.len() {
        return Err("cargo metadata contains duplicate package identifiers".to_owned());
    }
    let members_value = check_try!(
        to_value(members)
            .map_err(|error| return format!("serialize lock-wide package roots: {error}"))
    );
    let replaced_members = metadata
        .other
        .insert("workspace_members".to_owned(), members_value.clone());
    let replaced_defaults = metadata
        .other
        .insert("workspace_default_members".to_owned(), members_value);
    if replaced_members.is_none() || replaced_defaults.is_none() {
        return Err("cargo metadata omitted workspace member roots".to_owned());
    }
    return to_vec(&metadata)
        .map_err(|error| return format!("serialize lock-wide cargo metadata: {error}"));
}

/// Parse one Cargo metadata document.
///
/// # Errors
///
/// Returns an error when its schema is malformed or incomplete.
fn parse_metadata(source: &[u8]) -> CheckResult<MetadataSnapshot> {
    return from_slice::<MetadataSnapshot>(source)
        .map_err(|error| return format!("parse cargo metadata: {error}"));
}

/// Remove every metadata package and edge absent from the Cargo tree.
///
/// # Errors
///
/// Returns an error when the tree and metadata roots or edges disagree.
fn prune_metadata(
    mut metadata: MetadataSnapshot,
    graph: &ActiveGraph,
) -> CheckResult<MetadataSnapshot> {
    let root = check_try!(
        graph
            .root()
            .ok_or_else(|| return "active Cargo graph has no root".to_owned())
    );
    let resolve = check_try!(
        metadata
            .resolve
            .as_mut()
            .ok_or_else(|| return "cargo metadata did not return a resolve graph".to_owned())
    );
    if resolve.root.as_deref() != Some(root) {
        return Err(format!(
            "cargo metadata root {:?} does not match cargo tree root {root}",
            resolve.root
        ));
    }
    metadata
        .packages
        .retain(|package| return graph.contains(package.identifier.as_str()));
    resolve
        .nodes
        .retain(|node| return graph.contains(node.identifier.as_str()));
    for node in &mut resolve.nodes {
        let active = check_try!(
            graph
                .get(node.identifier.as_str())
                .ok_or_else(|| return format!("active package {} is missing", node.identifier))
        );
        check_try!(rewrite_node(node, active));
    }
    check_try!(validate_pruned(&metadata, graph));
    return Ok(metadata);
}

/// Prune one metadata document using a supplied Cargo tree source.
///
/// # Errors
///
/// Returns an error when either input is malformed or inconsistent.
#[cfg(test)]
pub(super) fn prune_metadata_source(
    metadata_json: &[u8],
    tree_source: &str,
) -> CheckResult<ActiveMetadata> {
    let metadata = check_try!(parse_metadata(metadata_json));
    let graph = check_try!(tree::parse_graph(&metadata, tree_source));
    return serialize_metadata(check_try!(prune_metadata(metadata, &graph)));
}

/// Replace one metadata node with Cargo tree edges and features.
///
/// # Errors
///
/// Returns an error when Cargo tree reports an edge absent from metadata.
fn rewrite_node(node: &mut ResolveNode, active: &ActiveNode) -> CheckResult {
    node.dependency_details.retain(|dependency| {
        return active
            .dependencies()
            .contains(dependency.package_identifier.as_str());
    });
    let detailed = node
        .dependency_details
        .iter()
        .map(|dependency| return dependency.package_identifier.as_str())
        .collect::<BTreeSet<_>>();
    if let Some(missing) = active
        .dependencies()
        .iter()
        .find(|dependency| return !detailed.contains(dependency.as_str()))
    {
        return Err(format!(
            "cargo tree edge {} -> {missing} is absent from metadata",
            node.identifier
        ));
    }
    node.dependencies = active.dependencies().iter().cloned().collect();
    node.features = active.features().iter().cloned().collect();
    return Ok(());
}

/// Serialize one pruned metadata document for cargo-deny.
///
/// # Errors
///
/// Returns an error when JSON serialization fails.
fn serialize_metadata(metadata: MetadataSnapshot) -> CheckResult<ActiveMetadata> {
    let serialized = check_try!(
        to_vec(&metadata)
            .map_err(|error| return format!("serialize pruned cargo metadata: {error}"))
    );
    return Ok(ActiveMetadata {
        metadata,
        serialized,
    });
}

/// Require the pruned package and node sets to equal the active graph.
///
/// # Errors
///
/// Returns an error when a package is missing, duplicated, or retained.
fn validate_pruned(metadata: &MetadataSnapshot, graph: &ActiveGraph) -> CheckResult {
    let packages = metadata
        .packages
        .iter()
        .map(|package: &MetadataPackage| return package.identifier.as_str())
        .collect::<BTreeSet<_>>();
    let resolve = check_try!(
        metadata
            .resolve
            .as_ref()
            .ok_or_else(|| return "pruned cargo metadata lost its resolve graph".to_owned())
    );
    let nodes = resolve
        .nodes
        .iter()
        .map(|node| return node.identifier.as_str())
        .collect::<BTreeSet<_>>();
    let active = graph.identifiers();
    let exact_counts = packages.len() == metadata.packages.len()
        && nodes.len() == resolve.nodes.len()
        && active.len() == graph.node_count();
    if exact_counts && packages == active && nodes == active {
        return Ok(());
    }
    return Err("pruned cargo metadata does not exactly match the active Cargo tree".to_owned());
}
