package main

import (
	"fmt"
	"path/filepath"
	"strings"
)

func checkDocs() {
	var docs docsJSON
	readJSON("docs/docs.json", &docs)

	var pages []string
	for _, tab := range docs.Navigation.Tabs {
		for _, group := range tab.Groups {
			for _, page := range group.Pages {
				pageName, ok := page.(string)
				if !ok {
					fail("docs navigation page entries must be strings")
				}
				pages = append(pages, pageName)
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

	navPages := strings.Join(pages, "\n")
	openapi := readText("docs/openapi.json")
	readme := readText("README.md")
	pricing := readText("docs/pricing.mdx")
	scrapers := readText("docs/scrapers.mdx")
	agents := readText("docs/agents.mdx")
	packages := readText("docs/reference/packages.mdx")
	llms := readText("docs/llms.txt")
	skill := readText("docs/skill.md")
	packagedSkill := readText("skills/tovuk/SKILL.md")
	status := readText("docs/status.mdx")
	abuse := readText("docs/abuse.mdx")
	support := readText("docs/support.mdx")

	publicCopy := strings.Join([]string{
		readme,
		openapi,
		readText("docs/docs.json"),
		llms,
		skill,
		strings.Join(readSortedTextsRecursive("docs", ".mdx"), "\n"),
		readText("crates/tovuk/README.md"),
		readText("packages/tovuk/README.md"),
		readText("packages/tovuk/package.json"),
		readText("packages/tovuk-py/README.md"),
		readText("packages/tovuk-py/pyproject.toml"),
		packagedSkill,
	}, "\n")
	rejectForbiddenPublicCopyTerms("public docs and package copy", publicCopy)

	for _, page := range []string{
		"index",
		"quickstart",
		"scrapers",
		"agents",
		"pricing",
		"status",
		"support",
		"abuse",
		"changelog",
		"reference/packages",
	} {
		requireContains(navPages, page, "Mintlify scraper-only navigation "+page)
	}
	for _, page := range []string{
		"deploy",
		"templates",
		"production-readiness",
		"reference/project-contract",
		"reference/workers",
		"reference/resources",
		"reference/sqlite",
		"reference/state",
		"reference/kv",
		"reference/secrets",
		"reference/storage",
		"reference/queues",
		"reference/cron",
		"reference/bindings",
		"reference/domains",
		"reference/logs-builds",
		"reference/usage-caps",
	} {
		rejectContains(navPages, page, "retired Mintlify navigation "+page)
	}

	for _, source := range []struct {
		name string
		text string
	}{
		{"README", readme},
		{"scraper docs", scrapers},
		{"agents", agents},
		{"packages", packages},
		{"llms", llms},
		{"docs skill", skill},
		{"packaged skill", packagedSkill},
	} {
		requireContains(source.text, `tovuk request create tiktok`, source.name+" TikTok example")
		requireContains(source.text, `tovuk request create github`, source.name+" GitHub example")
		requireContains(source.text, `tovuk request create linkedin`, source.name+" LinkedIn example")
		requireContains(source.text, "public data only", source.name+" public-data policy")
	}

	requireContains(status, "tovuk scraper health --json", "status scraper health docs")
	requireContains(support, "tovuk support create", "support create docs")
	requireContains(abuse, "tovuk abuse quarantine", "abuse operator docs")
	requireContains(pricing, "There is no free scraper tier", "pricing paid-only scraper docs")
	requireContains(pricing, "| Pro | `$20/month` | `$20`", "pricing Pro balance docs")
	requireContains(pricing, "| Business | `$100/month` | `$125`", "pricing Business balance docs")
	requireContains(pricing, "| Scale | `$200/month` | `$300`", "pricing Scale balance docs")
	requireContains(pricing, "deducts from that balance for each successful stored", "pricing balance debit docs")
	requireContains(pricing, "`priceEvents[].usdMicros`", "pricing scraper event price docs")
	requireContains(pricing, "| Google Maps Scraper | place | `$2.10` |", "pricing Google Maps per-result docs")
	requireContains(pricing, "| TikTok Scraper | record | `$1.70` |", "pricing TikTok per-result docs")
	requireContains(pricing, "| Instagram Scraper | record | `$0.80` |", "pricing Instagram per-result docs")

	for _, path := range []string{
		`"/health"`,
		`"/healthz"`,
		`"/v1/status"`,
		`"/v1/login/device"`,
		`"/v1/login/device/{device_code}"`,
		`"/v1/account"`,
		`"/v1/account/activity"`,
		`"/v1/scrapers"`,
		`"/v1/scrapers/health"`,
		`"/v1/scrapers/{scraper}"`,
		`"/v1/requests"`,
		`"/v1/requests/{request_id}"`,
		`"/v1/requests/{request_id}/cancel"`,
		`"/v1/requests/{request_id}/results"`,
		`"/v1/usage"`,
		`"/v1/billing/checkout"`,
		`"/v1/billing/portal"`,
		`"/v1/support/tickets"`,
		`"/v1/support/tickets/{ticket_id}/resolve"`,
		`"/v1/abuse/reports"`,
		`"/v1/abuse/reports/{report_id}/appeal"`,
		`"/v1/operator/abuse/reports"`,
	} {
		requireContains(openapi, path, "OpenAPI scraper-only path "+path)
	}

	for _, retired := range []string{
		`"/v1/apps"`,
		`"/v1/deploy"`,
		`"/v1/deploys"`,
		`"/v1/services"`,
		`"/v1/builds"`,
		`"/v1/capabilities"`,
		`"/v1/usage/caps`,
		"DeployRequest",
		"DeployResponse",
		"ServicesResponse",
		"ServiceOverviewResponse",
		"StorageObjectsResponse",
		"SqliteQueryResponse",
		"QueueMessageSendRequest",
		"CronTrigger",
		"UsageCap",
		"TovukConfig",
	} {
		rejectContains(openapi, retired, "retired public OpenAPI contract "+retired)
	}

	for _, retired := range []string{
		"tovuk deploy",
		"tovuk service",
		"tovuk storage",
		"tovuk sqlite",
		"tovuk kv",
		"tovuk queue",
		"tovuk cron",
		"tovuk secrets",
		"tovuk domains",
		"tovuk limits",
		"tovuk nodes",
		"tovuk.toml",
		"full-stack",
		"static frontend",
	} {
		rejectContains(publicCopy, retired, "retired public docs wording "+retired)
	}

	requireContains(openapi, `"linkedinPostSearch"`, "OpenAPI LinkedIn post search example")
	requireContains(openapi, `"author_company_urns"`, "OpenAPI LinkedIn author company filter")
	requireContains(openapi, `"linkedinCompanyEmployees"`, "OpenAPI LinkedIn company employees example")

	fmt.Println("Checked scraper-only docs, package copy, and OpenAPI contract.")
}
