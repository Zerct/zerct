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
		readSortedTextsRecursive("crates/tovuk/src/cli", ".rs")...,
	), "\n")

	rootReadme := readText("README.md")
	cargoReadme := readText("crates/tovuk/README.md")
	npmPackage := readPackageJSON("packages/tovuk/package.json")
	npmInstall := readText("packages/tovuk/install.mjs")
	npmReadme := readText("packages/tovuk/README.md")
	pythonCLI := readText("packages/tovuk-py/src/tovuk/cli.py")
	pythonReadme := readText("packages/tovuk-py/README.md")
	pythonProject := readText("packages/tovuk-py/pyproject.toml")
	homebrewFormula := readText("Formula/tovuk.rb")

	for _, command := range []string{
		"login", "account", "pricing", "scraper", "request", "usage",
		"billing", "support", "abuse",
	} {
		requireContains(cargoCLI, fmt.Sprintf("%q", command), fmt.Sprintf("native command %s", command))
	}

	publicSources := []string{
		rootReadme,
		cargoReadme,
		npmReadme,
		pythonReadme,
		homebrewFormula,
		readText("docs/index.mdx"),
		readText("docs/quickstart.mdx"),
		readText("docs/agents.mdx"),
		readText("docs/reference/packages.mdx"),
		readText("docs/llms.txt"),
		readText("docs/skill.md"),
		readText("skills/tovuk/SKILL.md"),
	}
	coreCommands := []string{
		"tovuk account show",
		"tovuk pricing",
		"tovuk scraper list",
		"tovuk scraper health",
		"tovuk scraper show",
		"tovuk request create",
		"tovuk request show",
		"tovuk request results",
		"tovuk usage",
		"tovuk billing checkout",
		"tovuk billing portal",
	}
	for _, source := range append([]string{cargoCLI}, publicSources...) {
		for _, snippet := range coreCommands {
			requireContains(source, snippet, "scraper-only public command "+snippet)
		}
	}

	fullWorkflowSources := []string{
		rootReadme,
		readText("docs/agents.mdx"),
		readText("docs/reference/packages.mdx"),
		readText("docs/llms.txt"),
		readText("skills/tovuk/SKILL.md"),
		cargoCLI,
	}
	for _, source := range fullWorkflowSources {
		for _, snippet := range []string{
			"tovuk support create",
			"tovuk support list",
			"tovuk support resolve",
			"tovuk abuse report",
			"tovuk abuse list",
			"tovuk abuse list --operator",
			"tovuk abuse appeal",
			"tovuk abuse triage",
			"tovuk abuse notify-owner",
			"tovuk abuse quarantine",
			"tovuk abuse resolve",
			"tovuk abuse reject",
			"tovuk abuse release",
		} {
			requireContains(source, snippet, "scraper-only public command "+snippet)
		}
	}

	installGuideSources := []string{
		rootReadme,
		cargoReadme,
		npmReadme,
		pythonReadme,
		readText("docs/index.mdx"),
		readText("docs/quickstart.mdx"),
		readText("docs/reference/packages.mdx"),
		readText("docs/llms.txt"),
	}
	for _, source := range installGuideSources {
		requireContains(source, "brew tap tovuk/tovuk https://github.com/tovuk/tovuk", "main-repo Homebrew tap command")
		requireContains(source, "brew install tovuk", "simple Homebrew install command")
		requireContains(source, "public data only", "public-data boundary")
	}

	requireContains(npmInstall, "TOVUK_NATIVE_BINARY", "npm local native binary override")
	requireContains(pythonCLI, "TOVUK_NATIVE_BINARY", "PyPI local native binary override")
	requireContains(homebrewFormula, `depends_on "rust" => :build`, "Homebrew builds native Rust CLI")
	requireContains(homebrewFormula, "crates/tovuk", "Homebrew installs Rust crate path")
	requireEqual(npmPackage.Description, "Use Tovuk scraper APIs from a native CLI.", "npm package description")
	requireContains(pythonProject, `description = "Use Tovuk scraper APIs from a native CLI."`, "PyPI package description")

	if npmPackage.Bin["tovuk"] != "bin/tovuk" {
		fail("npm package must expose bin/tovuk")
	}
	if len(npmPackage.Dependencies) > 0 || len(npmPackage.DevDependencies) > 0 {
		fail("npm package must not ship runtime JavaScript dependencies")
	}

	retiredOrgScope := string([]byte{64, 122, 101, 114, 99, 116})
	retiredHomebrewTap := "tovuk" + "/tap"
	retiredQualifiedHomebrew := "brew install " + "tovuk" + "/tovuk/tovuk"
	for _, source := range []string{cargoCLI, npmInstall, pythonCLI, cargoReadme, npmReadme, pythonReadme, homebrewFormula} {
		rejectContains(source, "TOVUK_NPM_CLI", "retired npm delegation")
		rejectContains(source, "NPM_PACKAGE_VERSION", "retired npm package pin")
		rejectContains(source, "npx -y", "retired npx delegation")
		rejectContains(source, retiredHomebrewTap, "retired archived Homebrew tap")
		rejectContains(source, retiredQualifiedHomebrew, "retired qualified Homebrew install")
		rejectContains(source, "--app", "retired app flag")
		rejectContains(source, "/v1/apps", "retired apps API path")
		rejectContains(source, retiredOrgScope, "retired org scope")
	}

	retiredCommands := []string{
		"tovuk new", "tovuk check", "tovuk dev", "tovuk deploy", "tovuk service",
		"tovuk logs", "tovuk sqlite", "tovuk kv", "tovuk queue", "tovuk cron",
		"tovuk state", "tovuk binding", "tovuk limits", "tovuk env",
		"tovuk secrets", "tovuk domains", "tovuk storage", "tovuk nodes",
	}
	retiredCommandSources := []string{
		cargoCLI,
		rootReadme,
		cargoReadme,
		npmReadme,
		pythonReadme,
		readText("docs/index.mdx"),
		readText("docs/quickstart.mdx"),
		readText("docs/agents.mdx"),
		readText("docs/reference/packages.mdx"),
		readText("docs/llms.txt"),
		readText("docs/skill.md"),
		readText("skills/tovuk/SKILL.md"),
	}
	for _, source := range retiredCommandSources {
		for _, command := range retiredCommands {
			rejectContains(source, command, "retired public command "+command)
		}
	}

	for _, source := range publicSources {
		for _, snippet := range []string{
			"tovuk.toml",
			"full-stack",
			"static frontend",
			"deploy workflow",
			"deploy failed",
			"service snapshot",
			"build id",
			"build logs",
			"usage caps",
		} {
			rejectContains(strings.ToLower(source), snippet, "retired deploy-platform wording "+snippet)
		}
	}

	fmt.Println("Checked scraper-only native CLI command and package contract.")
}
