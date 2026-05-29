package main

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"sort"
	"strconv"
	"strings"
	"time"
)

type packageJSON struct {
	Bin             map[string]string `json:"bin"`
	Bugs            map[string]string `json:"bugs"`
	Dependencies    map[string]string `json:"dependencies"`
	Description     string            `json:"description"`
	DevDependencies map[string]string `json:"devDependencies"`
	Engines         map[string]string `json:"engines"`
	Files           []string          `json:"files"`
	Homepage        string            `json:"homepage"`
	License         string            `json:"license"`
	Name            string            `json:"name"`
	Private         *bool             `json:"private"`
	PublishConfig   map[string]string `json:"publishConfig"`
	Repository      map[string]string `json:"repository"`
	Scripts         map[string]string `json:"scripts"`
	Type            string            `json:"type"`
	Version         string            `json:"version"`
}

type docsJSON struct {
	Navigation struct {
		Tabs []struct {
			Groups []struct {
				Pages []interface{} `json:"pages"`
			} `json:"groups"`
		} `json:"tabs"`
	} `json:"navigation"`
}

func main() {
	if len(os.Args) < 2 {
		fail("usage: go run scripts/check-public-contracts.go <check>")
	}
	repoRoot := findRepoRoot()
	if err := os.Chdir(repoRoot); err != nil {
		fail("cd %s: %v", repoRoot, err)
	}

	switch os.Args[1] {
	case "package-versions":
		checkPackageVersions()
	case "cli-contract":
		checkCLIContract()
	case "docs":
		checkDocs()
	case "npm-cli-package":
		checkNPMCLIPackage()
	case "npm-native-runtime":
		checkNPMNativeRuntime()
	case "mintlify-agent-readiness":
		target := "https://docs.tovuk.com"
		if len(os.Args) > 2 {
			target = os.Args[2]
		}
		checkMintlifyAgentReadiness(target)
	case "mintlify-score":
		if len(os.Args) != 3 {
			fail("mintlify-score requires a score JSON path")
		}
		checkMintlifyScore(os.Args[2])
	case "npm-version":
		fmt.Println(readPackageJSON("packages/tovuk/package.json").Version)
	default:
		fail("unknown check %q", os.Args[1])
	}
}

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
		"init", "install", "doctor", "preview", "login", "deploy", "capabilities",
		"me", "usage", "activity", "apps", "overview", "deploys", "builds", "logs",
		"status", "inspect", "db", "database", "env", "domains", "billing", "support",
	} {
		requireContains(cargoCLI, fmt.Sprintf("%q", command), fmt.Sprintf("native command %s", command))
	}

	for _, source := range []string{cargoCLI, cargoReadme, npmReadme, pythonReadme, homebrewFormula} {
		requireContains(source, "tovuk billing checkout --json", "agentic billing checkout command")
		requireContains(source, "tovuk support create", "agentic support create command")
		requireContains(source, "tovuk support list", "agentic support list command")
		requireContains(source, "tovuk support resolve", "agentic support resolve command")
	}

	requireContains(cargoCLI, "fullstack-rust-tanstack", "fullstack template option")
	requireContains(cargoCLI, "tanstack-static-frontend", "frontend template option")
	requireContains(cargoCLI, "rust-api", "Rust template option")
	requireContains(cargoCLI, "JavaScript and TypeScript are frontend-only on Tovuk", "Rust-only backend policy")
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
		rejectContains(source, retiredOrgScope, "retired org scope")
	}

	fmt.Println("Checked native CLI command and package contract.")
}

func checkDocs() {
	var docs docsJSON
	readJSON("docs/docs.json", &docs)

	var pages []string
	for _, tab := range docs.Navigation.Tabs {
		for _, group := range tab.Groups {
			for _, page := range group.Pages {
				collectPageEntry(page, &pages)
			}
		}
	}

	var missingPages []string
	for _, page := range pages {
		if strings.HasPrefix(page, "http://") || strings.HasPrefix(page, "https://") {
			continue
		}
		pagePath := filepath.Join("docs", page+".mdx")
		if !fileExists(pagePath) {
			missingPages = append(missingPages, pagePath)
		}
	}

	if len(missingPages) > 0 {
		fail("Missing Mintlify pages:\n%s", strings.Join(missingPages, "\n"))
	}

	fmt.Printf("Checked %d Mintlify navigation entries.\n", len(pages))
}

func checkNPMCLIPackage() {
	repoRoot := mustAbs(".")
	packageDir := filepath.Join(repoRoot, "packages", "tovuk")
	packageJSONPath := filepath.Join(packageDir, "package.json")
	installPath := filepath.Join(packageDir, "install.mjs")
	binPath := filepath.Join(packageDir, "bin", "tovuk")

	requiredFiles := []string{"bin", "install.mjs", "README.md"}
	requiredPackageScripts := map[string]string{
		"check":        "npm run check:policy && npm run runtime && npm run pack:dry",
		"check:policy": "go run ../../scripts/check-public-contracts.go npm-cli-package",
		"pack:dry":     "npm pack --dry-run",
		"postinstall":  "node install.mjs",
		"runtime":      "go run ../../scripts/check-public-contracts.go npm-native-runtime",
	}

	packageJSON := readPackageJSON(packageJSONPath)
	requireEqual(packageJSON.Name, "tovuk", "package name")
	requireEqual(packageJSON.Type, "module", "package type")
	requireEqual(packageJSON.Description, "Deploy Rust backends, static frontends, and fullstack apps to Tovuk.", "package description")
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

func checkMintlifyAgentReadiness(target string) {
	baseURL := normalizeTargetURL(target)
	retries := envInt("TOVUK_DOCS_CHECK_RETRIES", 8)
	retryDelay := time.Duration(envInt("TOVUK_DOCS_CHECK_RETRY_DELAY_MS", 5000)) * time.Millisecond
	client := &http.Client{Timeout: 20 * time.Second}

	requiredPaths := []string{
		"/llms.txt",
		"/llms-full.txt",
		"/skill.md",
		"/.well-known/skills/index.json",
		"/.well-known/agent-skills/index.json",
		"/.well-known/mcp",
		"/sitemap.xml",
		"/robots.txt",
		"/openapi.json",
	}

	for _, path := range requiredPaths {
		response := fetchText(client, baseURL, path, nil, retries, retryDelay)
		if strings.TrimSpace(response) == "" {
			fail("%s is empty", path)
		}
	}

	llms := fetchText(client, baseURL, "/llms.txt", nil, retries, retryDelay)
	requirePattern("llms.txt", llms, `(?m)^# `)
	requirePattern("llms.txt", llms, `\[[^\]]+\]\([^)]+\)`)

	skill := fetchText(client, baseURL, "/skill.md", nil, retries, retryDelay)
	requirePattern("skill.md", skill, `(?m)^---\n`)
	requirePattern("skill.md", skill, `(?i)name:\s*tovuk`)

	robots := fetchText(client, baseURL, "/robots.txt", nil, retries, retryDelay)
	if regexp.MustCompile(`(?i)Disallow:\s*/`).MatchString(robots) &&
		!regexp.MustCompile(`(?i)Allow:\s*/`).MatchString(robots) {
		fail("robots.txt appears to block crawlers")
	}

	markdown := fetchText(client, baseURL, "/", map[string]string{"Accept": "text/markdown"}, retries, retryDelay)
	requirePattern("Markdown content negotiation", markdown, `Tovuk`)

	plaintext := fetchText(client, baseURL, "/", map[string]string{"Accept": "text/plain"}, retries, retryDelay)
	requirePattern("Plain text content negotiation", plaintext, `Tovuk`)

	mcpDiscovery := fetchText(client, baseURL, "/.well-known/mcp", nil, retries, retryDelay)
	requirePattern("MCP discovery", mcpDiscovery, `"url"\s*:`)
	requirePattern("MCP discovery", mcpDiscovery, `/mcp`)

	fmt.Printf("Mintlify agent readiness checks passed for %s\n", baseURL)
}

func checkMintlifyScore(path string) {
	var score map[string]interface{}
	readJSON(path, &score)

	value := numberField(score, "score")
	if value == 0 {
		value = numberField(score, "overallScore")
	}
	minimum := float64(envInt("MINTLIFY_SCORE_MIN", 90))
	if value < minimum {
		fail("Mintlify score is %.0f/100; expected at least %.0f/100", value, minimum)
	}
	fmt.Printf("Mintlify score is %.0f/100\n", value)
}

func collectPageEntry(entry interface{}, pages *[]string) {
	switch value := entry.(type) {
	case string:
		*pages = append(*pages, value)
	case map[string]interface{}:
		rawPages, ok := value["pages"].([]interface{})
		if !ok {
			return
		}
		for _, nestedEntry := range rawPages {
			collectPageEntry(nestedEntry, pages)
		}
	}
}

func fetchText(client *http.Client, baseURL string, path string, headers map[string]string, retries int, retryDelay time.Duration) string {
	var lastError error
	for attempt := 0; attempt <= retries; attempt++ {
		text, err := requestText(client, baseURL, path, headers)
		if err == nil {
			return text
		}
		lastError = err
		if attempt == retries || !isRetryableFetchError(err) {
			break
		}
		time.Sleep(retryDelay)
	}
	fail("%s", lastError)
	return ""
}

func requestText(client *http.Client, baseURL string, path string, headers map[string]string) (string, error) {
	request, err := http.NewRequest(http.MethodGet, baseURL+path, nil)
	if err != nil {
		return "", err
	}
	for name, value := range headers {
		request.Header.Set(name, value)
	}

	response, err := client.Do(request)
	if err != nil {
		return "", err
	}
	defer response.Body.Close()

	body, err := io.ReadAll(response.Body)
	if err != nil {
		return "", err
	}
	if response.StatusCode < 200 || response.StatusCode >= 300 {
		return "", httpStatusError{Path: path, Status: response.StatusCode}
	}
	return string(body), nil
}

type httpStatusError struct {
	Path   string
	Status int
}

func (errorValue httpStatusError) Error() string {
	return fmt.Sprintf("%s returned %d", errorValue.Path, errorValue.Status)
}

func isRetryableFetchError(err error) bool {
	var statusError httpStatusError
	if errors.As(err, &statusError) {
		return statusError.Status == http.StatusTooManyRequests || statusError.Status >= 500
	}
	return true
}

func normalizeTargetURL(target string) string {
	if !strings.HasPrefix(target, "http://") && !strings.HasPrefix(target, "https://") {
		target = "https://" + target
	}
	return strings.TrimRight(target, "/")
}

func readSortedTexts(directory string, suffix string) []string {
	entries, err := os.ReadDir(directory)
	if err != nil {
		fail("read directory %s: %v", directory, err)
	}

	var names []string
	for _, entry := range entries {
		if !entry.IsDir() && strings.HasSuffix(entry.Name(), suffix) {
			names = append(names, entry.Name())
		}
	}
	sort.Strings(names)

	texts := make([]string, 0, len(names))
	for _, name := range names {
		texts = append(texts, readText(filepath.Join(directory, name)))
	}
	return texts
}

func readText(path string) string {
	content, err := os.ReadFile(path)
	if err != nil {
		fail("read %s: %v", path, err)
	}
	return string(content)
}

func readJSON(path string, target interface{}) {
	content, err := os.ReadFile(path)
	if err != nil {
		fail("read %s: %v", path, err)
	}
	decoder := json.NewDecoder(bytes.NewReader(content))
	if err := decoder.Decode(target); err != nil {
		fail("parse %s: %v", path, err)
	}
}

func readPackageJSON(path string) packageJSON {
	var manifest packageJSON
	readJSON(path, &manifest)
	return manifest
}

func regexpMatch(source string, pattern string, label string) string {
	match := regexp.MustCompile(pattern).FindStringSubmatch(source)
	if len(match) < 2 {
		fail("could not read %s", label)
	}
	return match[1]
}

func requirePattern(label string, source string, pattern string) {
	if !regexp.MustCompile(pattern).MatchString(source) {
		fail("%s did not match %s", label, pattern)
	}
}

func requireContains(source string, snippet string, label string) {
	if !strings.Contains(source, snippet) {
		fail("%s is missing", label)
	}
}

func rejectContains(source string, snippet string, label string) {
	if strings.Contains(source, snippet) {
		fail("%s is present", label)
	}
}

func requireEqual(actual string, expected string, label string) {
	if actual != expected {
		fail("%s must be %q, got %q", label, expected, actual)
	}
}

func requireStringSliceExactly(actual []string, expected []string, label string) {
	sortedActual := append([]string(nil), actual...)
	sortedExpected := append([]string(nil), expected...)
	sort.Strings(sortedActual)
	sort.Strings(sortedExpected)
	if strings.Join(sortedActual, "\x00") != strings.Join(sortedExpected, "\x00") {
		fail("%s must have exactly %s; unexpected: %s; missing: %s",
			label,
			strings.Join(sortedExpected, ", "),
			strings.Join(difference(sortedActual, sortedExpected), ", "),
			strings.Join(difference(sortedExpected, sortedActual), ", "),
		)
	}
}

func requireStringMapKeysExactly(actual map[string]string, expected []string, label string) {
	requireStringSliceExactly(mapKeys(actual), expected, label)
}

func mapKeys(values map[string]string) []string {
	keys := make([]string, 0, len(values))
	for key := range values {
		keys = append(keys, key)
	}
	return keys
}

func difference(left []string, right []string) []string {
	rightSet := make(map[string]bool, len(right))
	for _, value := range right {
		rightSet[value] = true
	}

	var diff []string
	for _, value := range left {
		if !rightSet[value] {
			diff = append(diff, value)
		}
	}
	if len(diff) == 0 {
		return []string{"none"}
	}
	return diff
}

func numberField(values map[string]interface{}, name string) float64 {
	switch value := values[name].(type) {
	case float64:
		return value
	case json.Number:
		number, err := value.Float64()
		if err != nil {
			return 0
		}
		return number
	default:
		return 0
	}
}

func envInt(name string, fallback int) int {
	raw := strings.TrimSpace(os.Getenv(name))
	if raw == "" {
		return fallback
	}
	value, err := strconv.Atoi(raw)
	if err != nil {
		fail("%s must be an integer", name)
	}
	return value
}

func fileExists(path string) bool {
	_, err := os.Stat(path)
	return err == nil
}

func mustAbs(path string) string {
	absolute, err := filepath.Abs(path)
	if err != nil {
		fail("resolve %s: %v", path, err)
	}
	return absolute
}

func findRepoRoot() string {
	command := exec.Command("git", "rev-parse", "--show-toplevel")
	output, err := command.Output()
	if err != nil {
		return mustAbs(".")
	}
	return strings.TrimSpace(string(output))
}

func fail(format string, args ...interface{}) {
	_, _ = fmt.Fprintf(os.Stderr, format+"\n", args...)
	os.Exit(1)
}
