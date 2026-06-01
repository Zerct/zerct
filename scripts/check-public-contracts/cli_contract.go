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
		},
		readSortedTexts("crates/tovuk/src/cli", ".rs")...,
	), "\n")

	cargoReadme := readText("crates/tovuk/README.md")
	npmPackage := readPackageJSON("packages/tovuk/package.json")
	npmInstall := readText("packages/tovuk/install.mjs")
	npmReadme := readText("packages/tovuk/README.md")
	pythonCLI := readText("packages/tovuk-py/src/tovuk/cli.py")
	pythonReadme := readText("packages/tovuk-py/README.md")
	homebrewFormula := readText("Formula/tovuk.rb")

	for _, command := range []string{
		"init", "install", "check", "preview", "login", "deploy", "capabilities", "pricing",
		"me", "usage", "activity", "service", "services", "overview", "deploys", "builds", "logs",
		"status", "inspect", "platform", "kv", "queue", "cron", "state",
		"binding", "limit", "database", "env", "domains", "storage", "files", "media",
		"billing", "support",
	} {
		requireContains(cargoCLI, fmt.Sprintf("%q", command), fmt.Sprintf("native command %s", command))
	}

	for _, source := range []string{cargoCLI, cargoReadme, npmReadme, pythonReadme, homebrewFormula} {
		requireContains(source, "tovuk storage list", "agentic storage list command")
		requireContains(source, "tovuk storage upload", "agentic storage upload command")
		requireContains(source, "tovuk storage download", "agentic storage download command")
		requireContains(source, "tovuk storage delete", "agentic storage delete command")
		requireContains(source, "tovuk kv put", "agentic kv put command")
		requireContains(source, "tovuk kv get", "agentic kv get command")
		requireContains(source, "tovuk queue send", "agentic queue send command")
		requireContains(source, "tovuk pricing", "agentic pricing command")
		requireContains(source, "tovuk deploy --dry-run", "agentic deploy dry-run command")
		requireContains(source, "tovuk billing checkout --json", "agentic billing checkout command")
		requireContains(source, "tovuk support create", "agentic support create command")
		requireContains(source, "tovuk support list", "agentic support list command")
		requireContains(source, "tovuk support resolve", "agentic support resolve command")
	}
	for _, source := range []string{cargoReadme, npmReadme, pythonReadme} {
		requireContains(source, "billingEstimate", "agentic usage cost estimate docs")
	}

	requireContains(cargoCLI, "worker-static-rust-tanstack", "worker-static template option")
	requireContains(cargoCLI, "tanstack-static-frontend", "frontend template option")
	requireContains(cargoCLI, "rust-worker", "Rust worker template option")
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
	for _, source := range []string{cargoCLI, npmInstall, pythonCLI, cargoReadme, npmReadme, pythonReadme, homebrewFormula} {
		rejectContains(source, "TOVUK_NPM_CLI", "retired npm delegation")
		rejectContains(source, "NPM_PACKAGE_VERSION", "retired npm package pin")
		rejectContains(source, "npx -y", "retired npx delegation")
		rejectContains(source, "--app", "retired app flag")
		rejectContains(source, "/v1/apps", "retired apps API path")
		rejectContains(source, retiredOrgScope, "retired org scope")
	}

	fmt.Println("Checked native CLI command and package contract.")
}
