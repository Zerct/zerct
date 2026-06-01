package main

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
)

const publicContractsCommand = "go run ../../scripts/check-public-contracts/*.go"

func checkNPMCLIPackage() {
	repoRoot := mustAbs(".")
	packageDir := filepath.Join(repoRoot, "packages", "tovuk")
	packageJSONPath := filepath.Join(packageDir, "package.json")
	installPath := filepath.Join(packageDir, "install.mjs")
	binPath := filepath.Join(packageDir, "bin", "tovuk")

	requiredFiles := []string{"bin", "install.mjs", "README.md"}
	requiredPackageScripts := map[string]string{
		"check":        "npm run check:policy && npm run runtime && npm run pack:dry",
		"check:policy": publicContractsCommand + " npm-cli-package",
		"pack:dry":     "npm pack --dry-run",
		"postinstall":  "node install.mjs",
		"runtime":      publicContractsCommand + " npm-native-runtime",
	}

	packageJSON := readPackageJSON(packageJSONPath)
	requireEqual(packageJSON.Name, "tovuk", "package name")
	requireEqual(packageJSON.Type, "module", "package type")
	requireEqual(packageJSON.Description, "Deploy Rust workers, static frontends, and full-stack services to Tovuk.", "package description")
	requireEqual(packageJSON.Homepage, "https://tovuk.com", "package homepage")
	requireEqual(packageJSON.License, "MIT", "package license")
	if packageJSON.Private != nil {
		fail("package private flag must be omitted")
	}
	requireEqual(packageJSON.Engines["node"], ">=18.17", "Node engine")
	requireEqual(packageJSON.PublishConfig["access"], "public", "publish access")
	requireEqual(packageJSON.Repository["directory"], "packages/tovuk", "repository directory")
	requireEqual(packageJSON.Bin["tovuk"], "bin/tovuk", "tovuk bin path")
	if len(packageJSON.Dependencies) > 0 {
		fail("runtime dependencies must be omitted")
	}
	if len(packageJSON.DevDependencies) > 0 {
		fail("development dependencies must be omitted")
	}
	requireStringSliceExactly(packageJSON.Files, requiredFiles, "published files")

	for _, file := range requiredFiles {
		if !fileExists(filepath.Join(packageDir, file)) {
			fail("published file entry does not exist: %s", file)
		}
	}

	requireStringMapKeysExactly(packageJSON.Scripts, mapKeys(requiredPackageScripts), "scripts")
	for script, command := range requiredPackageScripts {
		requireEqual(packageJSON.Scripts[script], command, script+" script")
	}

	installSource := readText(installPath)
	for _, snippet := range []string{
		"https://github.com/tovuk/tovuk/releases/download",
		"TOVUK_NATIVE_BINARY",
		"aarch64-apple-darwin",
		"x86_64-unknown-linux-gnu",
		"x86_64-pc-windows-msvc",
	} {
		requireContains(installSource, snippet, "install.mjs "+snippet)
	}

	info, err := os.Stat(binPath)
	if err != nil {
		fail("stat bin/tovuk: %v", err)
	}
	if info.Mode()&0o111 == 0 {
		fail("bin/tovuk must stay executable")
	}

	fmt.Println("Checked npm native CLI package policy.")
}

func checkNPMNativeRuntime() {
	repoRoot := mustAbs(".")
	binary := os.Getenv("TOVUK_NATIVE_BINARY")
	if binary == "" {
		for _, candidate := range []string{
			filepath.Join(repoRoot, "crates", "tovuk", "target", "release", "tovuk"),
			filepath.Join(repoRoot, "packages", "tovuk", "bin", "tovuk"),
		} {
			if fileExists(candidate) {
				binary = candidate
				break
			}
		}
	}

	if binary == "" || !fileExists(binary) {
		fail("native Tovuk binary does not exist: %s", binary)
	}

	command := exec.Command(binary, "--version")
	command.Stdout = os.Stdout
	command.Stderr = os.Stderr
	if err := command.Run(); err != nil {
		fail("native Tovuk binary failed: %v", err)
	}
}
