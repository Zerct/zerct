//! Exact active Cargo tree discovery and parsing.

use alloc::collections::{BTreeMap, BTreeSet};

use core::{iter::once, str::from_utf8};

use std::process::{Command, Output};

use tovuk_public_checks::{
    check_support::{CheckResult, command},
    check_try,
};

use super::{MetadataSnapshot, graph::GraphContext};

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x000d] = [
    size_of_val(&append_tree_entry),
    size_of_val(&cargo_tree),
    size_of_val(&load_graph),
    size_of_val(&package_lookup),
    size_of_val(&parse_features),
    size_of_val(&parse_graph),
    size_of_val(&parse_package_display),
    size_of_val(&parse_tree_entry),
    size_of_val(&parse_tree_fields),
    size_of_val(&prepare_cargo_tree),
    size_of_val(&record_parent),
    size_of_val(&record_root),
    size_of_val(&resolve_identity),
];

/// Exact target-specific graph reported by Cargo tree.
#[derive(Debug, Default)]
pub(super) struct ActiveGraph {
    /// Active package nodes keyed by Cargo package identifier.
    nodes: BTreeMap<String, ActiveNode>,
    /// Single package selected as the tree root.
    root: Option<String>,
}

impl ActiveGraph {
    /// Report whether one package identifier is active.
    pub(super) fn contains(&self, identifier: &str) -> bool {
        return self.nodes.contains_key(identifier);
    }

    /// Return one active package node.
    pub(super) fn get(&self, identifier: &str) -> Option<&ActiveNode> {
        return self.nodes.get(identifier);
    }

    /// Collect every active package identifier.
    pub(super) fn identifiers(&self) -> BTreeSet<&str> {
        return self.nodes.keys().map(String::as_str).collect();
    }

    /// Count exact active package identifiers.
    pub(super) fn node_count(&self) -> usize {
        return self.nodes.len();
    }

    /// Return the selected Cargo tree root.
    pub(super) fn root(&self) -> Option<&str> {
        return self.root.as_deref();
    }
}

/// Active edges and features for one Cargo package.
#[derive(Debug, Default)]
pub(super) struct ActiveNode {
    /// Direct active dependency package identifiers.
    dependencies: BTreeSet<String>,
    /// Features active in the target build graph.
    features: BTreeSet<String>,
}

impl ActiveNode {
    /// Return direct active dependency package identifiers.
    pub(super) const fn dependencies(&self) -> &BTreeSet<String> {
        return &self.dependencies;
    }

    /// Return features active for this target.
    pub(super) const fn features(&self) -> &BTreeSet<String> {
        return &self.features;
    }
}

/// Parsed Cargo package display identity.
#[derive(Debug)]
struct PackageDisplay {
    /// Cargo package name.
    name: String,
    /// Cargo package version without its display prefix.
    version: String,
}

/// Cargo name/version pair to every matching package identifier.
type PackageLookup = BTreeMap<(String, String), Vec<String>>;

/// One delimiter-safe Cargo tree record.
#[derive(Debug)]
struct TreeEntry {
    /// Dependency depth within the expanded tree.
    depth: usize,
    /// Features active for this package occurrence.
    features: BTreeSet<String>,
    /// Parsed Cargo package display.
    package: PackageDisplay,
}

/// Delimiter-safe Cargo tree fields.
#[derive(Debug)]
struct TreeFields {
    /// Dependency depth source.
    depth: String,
    /// Active package features source.
    features: String,
    /// Cargo package display source.
    package: String,
}

/// Merge one expanded tree occurrence into the active graph.
///
/// # Errors
///
/// Returns an error for multiple roots or an invalid depth transition.
fn append_tree_entry(
    graph: &mut ActiveGraph,
    ancestors: &mut Vec<String>,
    entry: TreeEntry,
    identity: String,
) -> CheckResult {
    if entry.depth > ancestors.len() {
        return Err(format!(
            "cargo tree depth {} skipped its parent depth",
            entry.depth
        ));
    }
    if entry.depth == 0x0000 {
        check_try!(record_root(graph, identity.as_str()));
    } else {
        check_try!(record_parent(
            graph,
            ancestors,
            entry.depth,
            identity.as_str()
        ));
    }
    graph
        .nodes
        .entry(identity.clone())
        .or_default()
        .features
        .extend(entry.features);
    ancestors.truncate(entry.depth);
    ancestors.push(identity);
    return Ok(());
}

/// Run Cargo tree for one locked all-feature shipped-target graph.
///
/// # Errors
///
/// Returns an error when Cargo cannot produce the active graph.
fn cargo_tree(context: &GraphContext) -> CheckResult<Output> {
    let output = check_try!(prepare_cargo_tree(context).output().map_err(|error| {
        return format!("run cargo tree for {}: {error}", context.manifest());
    }));
    if output.status.success() {
        return Ok(output);
    }
    return Err(format!(
        "cargo tree for {} {} failed with status {}:\n{}",
        context.manifest(),
        context.triple(),
        output.status,
        String::from_utf8_lossy(output.stderr.as_slice())
    ));
}

/// Load one exact active Cargo graph for a shipped target.
///
/// # Errors
///
/// Returns an error when Cargo fails or emits malformed output.
pub(super) fn load_graph(
    context: &GraphContext,
    metadata: &MetadataSnapshot,
) -> CheckResult<ActiveGraph> {
    let output = check_try!(cargo_tree(context));
    let source = check_try!(from_utf8(output.stdout.as_slice()).map_err(|error| {
        return format!(
            "cargo tree for {} {} was not UTF-8: {error}",
            context.manifest(),
            context.triple()
        );
    }));
    return parse_graph(metadata, source);
}

/// Index Cargo metadata packages by the identity Cargo tree displays.
fn package_lookup(metadata: &MetadataSnapshot) -> PackageLookup {
    let mut lookup = PackageLookup::new();
    for package in &metadata.packages {
        lookup
            .entry((package.name.clone(), package.version.clone()))
            .or_default()
            .push(package.identifier.clone());
    }
    return lookup;
}

/// Parse Cargo's comma-separated active feature set.
///
/// # Errors
///
/// Returns an error when an empty or padded feature appears.
fn parse_features(source: &str) -> CheckResult<BTreeSet<String>> {
    if source.is_empty() {
        return Ok(BTreeSet::new());
    }
    let features = source.split(',').collect::<Vec<_>>();
    if features
        .iter()
        .any(|feature| return feature.is_empty() || feature.trim() != *feature)
    {
        return Err("cargo tree contains an invalid feature name".to_owned());
    }
    return Ok(features.into_iter().map(str::to_owned).collect());
}

/// Parse the expanded Cargo tree into exact active nodes and edges.
///
/// # Errors
///
/// Returns an error when a record is malformed or cannot map uniquely.
pub(super) fn parse_graph(metadata: &MetadataSnapshot, source: &str) -> CheckResult<ActiveGraph> {
    let lookup = package_lookup(metadata);
    let mut graph = ActiveGraph::default();
    let mut ancestors = Vec::new();
    for (line_number, line) in source.lines().enumerate() {
        let entry = check_try!(parse_tree_entry(line).map_err(|error| {
            return format!("cargo tree record {line_number}: {error}");
        }));
        let identity = check_try!(resolve_identity(
            &lookup,
            entry.package.name.as_str(),
            entry.package.version.as_str()
        ));
        check_try!(append_tree_entry(
            &mut graph,
            &mut ancestors,
            entry,
            identity
        ));
    }
    if graph.root.is_none() {
        return Err("cargo tree returned no package records".to_owned());
    }
    return Ok(graph);
}

/// Parse Cargo's package display into an exact name/version pair.
///
/// # Errors
///
/// Returns an error when the display shape or version prefix is invalid.
fn parse_package_display(source: &str) -> CheckResult<PackageDisplay> {
    let mut fields = source.split_whitespace();
    let name = check_try!(
        fields
            .next()
            .ok_or_else(|| return "cargo tree package name is missing".to_owned())
    );
    let prefixed_version = check_try!(
        fields
            .next()
            .ok_or_else(|| return "cargo tree package version is missing".to_owned())
    );
    let version = check_try!(
        prefixed_version
            .strip_prefix('v')
            .filter(|stripped| return !stripped.is_empty())
            .ok_or_else(|| return "cargo tree package version lacks its v prefix".to_owned())
    );
    let suffix = fields.collect::<Vec<_>>().join(" ");
    if !(suffix.is_empty() || suffix.starts_with('(') && suffix.ends_with(')')) {
        return Err("cargo tree package source suffix is malformed".to_owned());
    }
    return Ok(PackageDisplay {
        name: name.to_owned(),
        version: version.to_owned(),
    });
}

/// Parse one custom-formatted Cargo tree record.
///
/// # Errors
///
/// Returns an error when delimiters, depth, package, or features are invalid.
fn parse_tree_entry(line: &str) -> CheckResult<TreeEntry> {
    let fields = check_try!(parse_tree_fields(line));
    let depth = check_try!(
        fields
            .depth
            .parse::<usize>()
            .map_err(|error| return format!("invalid cargo tree depth: {error}"))
    );
    let features = check_try!(parse_features(fields.features.as_str()));
    let package = check_try!(parse_package_display(fields.package.as_str()));
    return Ok(TreeEntry {
        depth,
        features,
        package,
    });
}

/// Split one Cargo tree record into exactly three fields.
///
/// # Errors
///
/// Returns an error when any field is absent or an extra delimiter exists.
fn parse_tree_fields(line: &str) -> CheckResult<TreeFields> {
    let mut fields = line.split('\t');
    let depth = check_try!(
        fields
            .next()
            .ok_or_else(|| return "cargo tree depth is missing".to_owned())
    );
    let package = check_try!(
        fields
            .next()
            .ok_or_else(|| return "cargo tree package is missing".to_owned())
    );
    let features = check_try!(
        fields
            .next()
            .ok_or_else(|| return "cargo tree features are missing".to_owned())
    );
    if fields.next().is_some() {
        return Err("cargo tree record has unexpected delimiters".to_owned());
    }
    return Ok(TreeFields {
        depth: depth.to_owned(),
        features: features.to_owned(),
        package: package.to_owned(),
    });
}

/// Prepare the canonical Cargo tree command without executing it.
fn prepare_cargo_tree(context: &GraphContext) -> Command {
    let mut prepared = command(context.repository(), context.path(), "cargo");
    let _: &mut Command = prepared.args([
        "tree",
        "--locked",
        "--manifest-path",
        context.manifest(),
        "--all-features",
        "--target",
        context.triple(),
        "--edges",
        "normal,build,dev",
        "--prefix",
        "depth",
        "--no-dedupe",
        "--format",
        "\t{p}\t{f}",
    ]);
    return prepared;
}

/// Record one non-root package edge using the depth ancestor stack.
///
/// # Errors
///
/// Returns an error when the parent depth is absent.
fn record_parent(
    graph: &mut ActiveGraph,
    ancestors: &[String],
    depth: usize,
    identity: &str,
) -> CheckResult {
    let parent_depth = check_try!(
        depth
            .checked_sub(0x0001)
            .ok_or_else(|| return "cargo tree parent depth underflowed".to_owned())
    );
    let parent = check_try!(
        ancestors
            .get(parent_depth)
            .ok_or_else(|| return "cargo tree parent record is missing".to_owned())
    );
    graph
        .nodes
        .entry(parent.clone())
        .or_default()
        .dependencies
        .extend(once(identity.to_owned()));
    return Ok(());
}

/// Record the single Cargo tree root.
///
/// # Errors
///
/// Returns an error when a second root appears.
fn record_root(graph: &mut ActiveGraph, identity: &str) -> CheckResult {
    if graph.root.is_some() {
        return Err("cargo tree returned more than one root".to_owned());
    }
    graph.root = Some(identity.to_owned());
    return Ok(());
}

/// Resolve a Cargo tree name/version pair to one exact metadata identifier.
///
/// # Errors
///
/// Returns an error when the pair is absent or ambiguous across sources.
fn resolve_identity(lookup: &PackageLookup, name: &str, version: &str) -> CheckResult<String> {
    let candidates = check_try!(
        lookup
            .get(&(name.to_owned(), version.to_owned()))
            .ok_or_else(|| return format!(
                "cargo tree package {name}@{version} is absent from metadata"
            ))
    );
    if candidates.len() != 0x0001 {
        return Err(format!(
            "cargo tree package {name}@{version} maps to {} metadata sources",
            candidates.len()
        ));
    }
    return candidates
        .first()
        .cloned()
        .ok_or_else(|| return format!("cargo tree package {name}@{version} has no identifier"));
}
