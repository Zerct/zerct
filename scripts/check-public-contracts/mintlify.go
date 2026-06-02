package main

import (
	"errors"
	"fmt"
	"io"
	"net/http"
	"regexp"
	"strings"
	"time"
)

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
		rejectRetiredPublicNames(path, response)
	}

	for _, path := range []string{"/", "/quickstart", "/pricing", "/reference/limits"} {
		response := fetchText(client, baseURL, path, map[string]string{"Accept": "text/html"}, retries, retryDelay)
		rejectRetiredPublicNames(path, response)
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

func rejectRetiredPublicNames(label string, source string) {
	lower := strings.ToLower(source)
	for _, retired := range retiredPublicNames() {
		if strings.Contains(lower, retired) {
			fail("%s contains retired public branding", label)
		}
	}
	rejectForbiddenPublicCopyTerms(label, source)
}

func retiredPublicNames() []string {
	return []string{
		string([]byte{122, 101, 114, 99, 116}),
		string([]byte{120, 113, 117, 105, 107}),
	}
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
