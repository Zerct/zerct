package main

import "fmt"

func checkPackageVersions() {
	npmPackage := readPackageJSON("packages/tovuk/package.json")
	pyproject := readText("packages/tovuk-py/pyproject.toml")
	pyInit := readText("packages/tovuk-py/src/tovuk/__init__.py")
	pythonCLI := readText("packages/tovuk-py/src/tovuk/cli.py")
	cargoToml := readText("crates/tovuk/Cargo.toml")
	cargoLock := readText("crates/tovuk/Cargo.lock")
	cargoCLIConstants := readText("crates/tovuk/src/cli/constants.rs")
	formula := readText("Formula/tovuk.rb")

	versions := map[string]string{
		"PyPI project":   regexpMatch(pyproject, `(?m)^version = "([^"]+)"`, "PyPI project version"),
		"Python package": regexpMatch(pyInit, `__version__ = "([^"]+)"`, "Python package version"),
		"Cargo.toml":     regexpMatch(cargoToml, `(?m)^version = "([^"]+)"`, "Cargo.toml version"),
		"Cargo.lock":     regexpMatch(cargoLock, `name = "tovuk"\nversion = "([^"]+)"`, "Cargo.lock version"),
		"Cargo CLI":      regexpMatch(cargoCLIConstants, `const VERSION: &str = "([^"]+)"`, "Cargo CLI version"),
	}

	for label, version := range versions {
		if version != npmPackage.Version {
			fail("%s %s does not match npm package %s", label, version, npmPackage.Version)
		}
	}

	requireContains(pythonCLI, "releases/download/v{__version__}", "Python native binary downloader release path")
	requireContains(pythonCLI, "tovuk-{__version__}-", "Python native binary downloader asset path")
	requireContains(formula, fmt.Sprintf(`tag: "v%s"`, npmPackage.Version), "Homebrew formula version tag")

	fmt.Println("Checked package version consistency.")
}
