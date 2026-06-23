use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PackageJson {
    #[serde(default)]
    pub(crate) bin: BTreeMap<String, String>,
    #[serde(default)]
    pub(crate) dependencies: BTreeMap<String, String>,
    #[serde(default)]
    pub(crate) description: String,
    #[serde(default)]
    pub(crate) dev_dependencies: BTreeMap<String, String>,
    #[serde(default)]
    pub(crate) engines: BTreeMap<String, String>,
    #[serde(default)]
    pub(crate) files: Vec<String>,
    #[serde(default)]
    pub(crate) homepage: String,
    #[serde(default)]
    pub(crate) license: String,
    #[serde(default)]
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) private: Option<bool>,
    #[serde(default)]
    pub(crate) publish_config: BTreeMap<String, String>,
    #[serde(default)]
    pub(crate) repository: BTreeMap<String, String>,
    #[serde(default)]
    pub(crate) scripts: BTreeMap<String, String>,
    #[serde(default, rename = "type")]
    pub(crate) package_type: String,
    #[serde(default)]
    pub(crate) version: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DocsJson {
    pub(crate) navigation: DocsNavigation,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DocsNavigation {
    #[serde(default)]
    pub(crate) tabs: Vec<DocsTab>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DocsTab {
    #[serde(default)]
    pub(crate) groups: Vec<DocsGroup>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DocsGroup {
    #[serde(default)]
    pub(crate) pages: Vec<Value>,
}
