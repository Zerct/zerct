package main

import (
	"fmt"
	"strings"
)

func checkCLIContract() {
	cargoCLI := strings.Join(append(
		[]string{
			readText("crates/tovuk/src/main.rs"),
			readText("crates/tovuk/src/cli.rs"),
			readText("crates/tovuk/src/cli/deploy/dry_run.rs"),
		},
		readSortedTextsRecursive("crates/tovuk/src/cli", ".rs")...,
	), "\n")

	rootReadme := readText("README.md")
	cargoReadme := readText("crates/tovuk/README.md")
	cargoTemplateCLI := readText("crates/tovuk/src/cli/templates/mod.rs")
	npmPackage := readPackageJSON("packages/tovuk/package.json")
	npmInstall := readText("packages/tovuk/install.mjs")
	npmReadme := readText("packages/tovuk/README.md")
	pythonCLI := readText("packages/tovuk-py/src/tovuk/cli.py")
	pythonReadme := readText("packages/tovuk-py/README.md")
	homebrewFormula := readText("Formula/tovuk.rb")

	for _, command := range []string{
		"new", "check", "login", "deploy", "pricing", "usage", "service", "logs",
		"sqlite", "kv", "queue", "cron", "state", "binding", "limits", "env",
		"domains", "storage", "billing", "support", "abuse", "nodes",
	} {
		requireContains(cargoCLI, fmt.Sprintf("%q", command), fmt.Sprintf("native command %s", command))
	}

	for _, source := range []string{cargoCLI, cargoReadme, npmReadme, pythonReadme, homebrewFormula} {
		requireContains(source, "tovuk storage list", "agentic storage list command")
		requireContains(source, "tovuk storage upload", "agentic storage upload command")
		requireContains(source, "tovuk storage download", "agentic storage download command")
		requireContains(source, "tovuk storage delete", "agentic storage delete command")
		requireContains(source, "tovuk sqlite create", "agentic SQLite create command")
		requireContains(source, "tovuk sqlite query", "agentic SQLite query command")
		requireContains(source, "tovuk sqlite backup", "agentic SQLite backup command")
		requireContains(source, "tovuk sqlite delete", "agentic SQLite delete command")
		requireContains(source, "tovuk kv put", "agentic kv put command")
		requireContains(source, "tovuk kv get", "agentic kv get command")
		requireContains(source, "tovuk queue send", "agentic queue send command")
		requireContains(source, "tovuk pricing", "agentic pricing command")
		requireContains(source, "tovuk deploy --dry-run", "agentic deploy dry-run command")
		requireContains(source, "tovuk deploy list", "agentic deploy list command")
		requireContains(source, "tovuk deploy show", "agentic deploy show command")
		requireContains(source, "tovuk deploy cancel", "agentic deploy cancel command")
		requireContains(source, "tovuk account show", "agentic account show command")
		requireContains(source, "tovuk account update", "agentic account update command")
		requireContains(source, "tovuk service show", "agentic service show command")
		requireContains(source, "tovuk billing checkout --json", "agentic billing checkout command")
		requireContains(source, "tovuk support create", "agentic support create command")
		requireContains(source, "tovuk support list", "agentic support list command")
		requireContains(source, "tovuk support resolve", "agentic support resolve command")
		requireContains(source, "tovuk abuse report", "agentic abuse report command")
		requireContains(source, "tovuk abuse list", "agentic abuse list command")
		requireContains(source, "tovuk abuse list --operator", "agentic operator abuse list command")
		requireContains(source, "tovuk abuse appeal", "agentic abuse appeal command")
		requireContains(source, "tovuk abuse triage", "agentic abuse triage command")
		requireContains(source, "tovuk abuse notify-owner", "agentic abuse owner notification command")
		requireContains(source, "tovuk abuse quarantine", "agentic abuse quarantine command")
		requireContains(source, "tovuk abuse resolve", "agentic abuse resolve command")
		requireContains(source, "tovuk abuse reject", "agentic abuse reject command")
		requireContains(source, "tovuk abuse release", "agentic abuse release command")
	}
	for _, source := range []string{cargoCLI, rootReadme, cargoReadme, npmReadme, pythonReadme} {
		requireContains(source, "tovuk nodes list", "operator node list command")
		requireContains(source, "tovuk nodes drain", "operator node drain command")
		requireContains(source, "tovuk nodes enable", "operator node enable command")
	}
	for _, source := range []string{cargoCLI, rootReadme, cargoReadme, npmReadme, pythonReadme} {
		requireContains(source, "tovuk storage url", "agentic storage URL command")
		requireContains(source, "tovuk state objects", "agentic State objects command")
		requireContains(source, "tovuk state keys", "agentic State keys command")
		requireContains(source, "tovuk state alarm set", "agentic State alarm set command")
		requireContains(source, "tovuk state alarm get", "agentic State alarm get command")
		requireContains(source, "tovuk state alarm delete", "agentic State alarm delete command")
		requireContains(source, "tovuk state delete-value", "agentic State value delete command")
	}
	for _, source := range []string{rootReadme, cargoReadme, npmReadme, pythonReadme} {
		requireContains(source, "controlled only by the committed `tovuk.toml`", "advisory scaffold wording")
		requireContains(source, "billingEstimate", "agentic usage cost estimate docs")
		requireContains(source, "meterPlan", "agentic deploy dry-run meter plan docs")
		requireContains(source, "enabled service meters", "agentic meter plan scope docs")
		requireContains(source, "ready-to-fill", "agentic cap command fill-in docs")
		requireContains(source, "`tovuk limits set`", "agentic cap command docs")
	}

	requireContains(cargoCLI, "fullstack-rust-tanstack", "full-stack template option")
	requireContains(cargoCLI, "meterPlan", "native deploy dry-run meter plan field")
	requireContains(cargoCLI, "capCommands", "native deploy dry-run cap commands field")
	requireContains(cargoCLI, `"/v1/account/activity"`, "native account activity route")
	requireContains(cargoCLI, "usage_caps_catalog_does_not_leak_disabled_resource_meters", "native deploy dry-run usage caps meter regression test")
	requireContains(cargoCLI, "tanstack-static-frontend", "frontend template option")
	requireContains(cargoCLI, "rust-worker", "Rust worker template option")
	requireContains(cargoTemplateCLI, "scaffolded {} config from existing files", "advisory new-project scaffold output")
	requireContains(cargoTemplateCLI, "deploy reads tovuk.toml only", "tovuk.toml-only scaffold guidance")
	requireContains(cargoCLI, "JavaScript and TypeScript are frontend-only on Tovuk", "Rust-only runtime policy")
	requireContains(npmInstall, "TOVUK_NATIVE_BINARY", "npm local native binary override")
	requireContains(pythonCLI, "TOVUK_NATIVE_BINARY", "PyPI local native binary override")
	requireContains(homebrewFormula, `depends_on "rust" => :build`, "Homebrew builds native Rust CLI")
	requireContains(homebrewFormula, "crates/tovuk", "Homebrew installs Rust crate path")

	if npmPackage.Bin["tovuk"] != "bin/tovuk" {
		fail("npm package must expose bin/tovuk")
	}
	if len(npmPackage.Dependencies) > 0 || len(npmPackage.DevDependencies) > 0 {
		fail("npm package must not ship runtime JavaScript dependencies")
	}

	retiredOrgScope := string([]byte{64, 122, 101, 114, 99, 116})
	retiredFullstackTemplate := "worker" + "-static-rust-tanstack"
	retiredFullstackKind := "kind = \"worker" + "_static\""
	retiredFullstackWording := "worker" + "-static"
	retiredDatabaseCommand := "tovuk " + "database"
	for _, source := range []string{cargoCLI, npmInstall, pythonCLI, cargoReadme, npmReadme, pythonReadme, homebrewFormula} {
		rejectContains(source, "TOVUK_NPM_CLI", "retired npm delegation")
		rejectContains(source, "NPM_PACKAGE_VERSION", "retired npm package pin")
		rejectContains(source, "npx -y", "retired npx delegation")
		rejectContains(source, "--app", "retired app flag")
		rejectContains(source, "/v1/apps", "retired apps API path")
		rejectContains(source, retiredOrgScope, "retired org scope")
	}
	for _, source := range []string{cargoCLI, cargoReadme, npmReadme, pythonReadme} {
		rejectContains(source, retiredFullstackTemplate, "retired full-stack template name")
		rejectContains(source, retiredFullstackKind, "retired full-stack project kind")
		rejectContains(source, retiredFullstackWording, "retired hyphenated full-stack wording")
		rejectContains(source, "tovuk init", "retired init command")
		rejectContains(source, "tovuk install", "retired install command")
		rejectContains(source, "tovuk preview", "retired preview command")
		rejectContains(source, "tovuk capabilities", "retired capabilities command")
		rejectContains(source, "tovuk me", "retired me command")
		rejectContains(source, "tovuk activity", "retired activity command")
		rejectContains(source, "tovuk overview", "retired overview command")
		rejectContains(source, "tovuk deploys", "retired top-level deploys command")
		rejectContains(source, "tovuk builds", "retired top-level builds command")
		rejectContains(source, "tovuk status", "retired top-level status command")
		rejectContains(source, "tovuk inspect", "retired top-level inspect command")
		rejectContains(source, "tovuk service inspect", "retired service inspect command")
		rejectContains(source, "tovuk service status", "retired service status command")
		rejectContains(source, "tovuk service resources", "retired service resources command")
		rejectContains(source, "tovuk service deploys", "retired service deploys command")
		rejectContains(source, "tovuk service builds", "retired service builds command")
		rejectContains(source, "tovuk platform", "retired platform command")
		rejectContains(source, "tovuk services", "retired services command")
		rejectContains(source, retiredDatabaseCommand, "retired database command")
		rejectContains(source, "tovuk caps", "retired caps command")
		rejectContains(source, "tovuk limit ", "retired singular limit command")
		rejectContains(source, "tovuk files", "retired storage alias")
		rejectContains(source, "tovuk media", "retired storage alias")
	}
	rejectContains(cargoCLI, `"/v1/activity"`, "retired account activity API route")

	fmt.Println("Checked native CLI command and package contract.")
}
