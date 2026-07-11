use alloc::collections::BTreeMap;

use serde::Deserialize;

use serde_json::Value;

#[derive(Debug, Default, Deserialize)]
/// Contract representation for `DocsApi`.
pub(super) struct DocsApi {
    #[serde(default)]
    /// Contract data stored in `openapi`.
    pub openapi: String,
}

#[derive(Debug, Deserialize)]
/// Contract representation for `DocsGroup`.
pub(super) struct DocsGroup {
    #[serde(default)]
    /// Contract data stored in `pages`.
    pub pages: Vec<Value>,
}

#[derive(Debug, Deserialize)]
/// Contract representation for `DocsJson`.
pub(super) struct DocsJson {
    #[serde(default)]
    /// Contract data stored in `api`.
    pub api: DocsApi,
    /// Contract data stored in `navigation`.
    pub navigation: DocsNavigation,
}

#[derive(Debug, Deserialize)]
/// Contract representation for `DocsNavigation`.
pub(super) struct DocsNavigation {
    #[serde(default)]
    /// Contract data stored in `tabs`.
    pub tabs: Vec<DocsTab>,
}

#[derive(Debug, Deserialize)]
/// Contract representation for `DocsTab`.
pub(super) struct DocsTab {
    #[serde(default)]
    /// Contract data stored in `groups`.
    pub groups: Vec<DocsGroup>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Contract representation for `PackageJson`.
pub(super) struct PackageJson {
    #[serde(default)]
    /// Contract data stored in `bin`.
    pub bin: BTreeMap<String, String>,
    #[serde(default)]
    /// Contract data stored in `dependencies`.
    pub dependencies: BTreeMap<String, String>,
    #[serde(default)]
    /// Contract data stored in `description`.
    pub description: String,
    #[serde(default)]
    /// Contract data stored in `dev_dependencies`.
    pub dev_dependencies: BTreeMap<String, String>,
    #[serde(default)]
    /// Contract data stored in `engines`.
    pub engines: BTreeMap<String, String>,
    #[serde(default)]
    /// Contract data stored in `files`.
    pub files: Vec<String>,
    #[serde(default)]
    /// Contract data stored in `homepage`.
    pub homepage: String,
    #[serde(default)]
    /// Contract data stored in `license`.
    pub license: String,
    #[serde(default)]
    /// Contract data stored in `name`.
    pub name: String,
    #[serde(default, rename = "type")]
    /// Contract data stored in `package_type`.
    pub package_type: String,
    #[serde(default)]
    /// Contract data stored in `private`.
    pub private: Option<bool>,
    #[serde(default)]
    /// Contract data stored in `publish_config`.
    pub publish_config: BTreeMap<String, String>,
    #[serde(default)]
    /// Contract data stored in `repository`.
    pub repository: BTreeMap<String, String>,
    #[serde(default)]
    /// Contract data stored in `scripts`.
    pub scripts: BTreeMap<String, String>,
    #[serde(default)]
    /// Contract data stored in `version`.
    pub version: String,
}
