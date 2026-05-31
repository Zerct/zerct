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

	openapi := readText("docs/openapi.json")
	pricing := readText("docs/pricing.mdx")
	limits := readText("docs/reference/limits.mdx")
	platform := readText("docs/reference/platform.mdx")
	products := readText("docs/reference/products.mdx")
	for _, retired := range []string{
		"/v1/apps",
		"--app",
		"app_id",
		"appId",
		"app_",
		"app_1",
		"targetApp",
		"alwaysOnApps",
		"appCount",
	} {
		rejectContains(openapi, retired, "retired public app contract")
	}

	requireContains(pricing, "`10 GB` per object", "Free State per-object storage docs")
	requireContains(limits, "State SQLite storage is 10 GB per object on Free", "State Free storage limit docs")
	requireContains(limits, "4.995 TiB object storage object size ceiling", "object storage max object docs")
	requireContains(limits, "5120 MiB (5 GiB)", "object storage single-part docs")
	requireContains(platform, "Free State objects get 10 GB per object", "platform State Free storage docs")
	requireContains(platform, "TOVUK_RUNTIME_TOKEN=tovuk_runtime_...", "runtime token binding docs")
	requireContains(platform, "TOVUK_SQLITE_DB=sqlite_1", "runtime SQLite binding docs")
	requireContains(platform, "TOVUK_BINDING_AUTH_SERVICE=service_2", "runtime service binding docs")
	requireContains(platform, "It cannot manage account settings", "runtime token scope docs")
	requireContains(products, "Use `products[].best_for`", "product choice docs")
	requireContains(products, "Worker:\nUse for Rust public APIs", "Worker product docs")
	requireContains(products, "State:\nUse for keyed realtime coordination", "State product docs")
	requireContains(products, "`state_duration_gb_milliseconds`", "State meter docs")
	requireContains(products, "runtime, storage, payload, batching", "limit field planning docs")
	requireContains(products, "Keep JavaScript and TypeScript frontend-only", "frontend-only product docs")
	requireContains(openapi, "build_minutes", "public OpenAPI build minutes meter")
	requireContains(openapi, `"workerDefaultCpuMsPerRequest"`, "OpenAPI worker default CPU limit field")
	requireContains(openapi, `"workerRequestBodySizeMib"`, "OpenAPI worker request body limit field")
	requireContains(openapi, `"workerWebsocketMessageMib"`, "OpenAPI worker websocket limit field")
	requireContains(openapi, `"stateSqliteStorageMib": 9536`, "OpenAPI State storage limit")
	requireContains(openapi, `"storageObjectMaxMib": 5237760`, "OpenAPI object size limit")
	requireContains(openapi, `"storageSinglePartUploadMaxMib": 5120`, "OpenAPI single-part upload limit")
	requireContains(limits, "tovuk limit set build_minutes", "build minutes cap docs")
	rejectContains(pricing, "`1 GB` per object", "stale Free State storage docs")
	rejectContains(limits, "State SQLite storage is 1 GB per object", "stale State storage docs")
	rejectContains(limits, "a 5 TiB object storage object size ceiling", "stale object storage object docs")
	rejectContains(limits, "5115 MiB", "stale object storage single-part docs")
	rejectContains(platform, "State objects get 1 GB per object", "stale platform State storage docs")
	rejectContains(openapi, `"stateSqliteStorageMib": 953,`, "stale OpenAPI State storage limit")
	rejectContains(openapi, `"storageObjectMaxMib": 5242880`, "stale OpenAPI object size limit")
	rejectContains(openapi, `"storageSinglePartUploadMaxMib": 5115`, "stale OpenAPI single-part upload limit")

	fmt.Printf("Checked %d Mintlify navigation entries.\n", len(pages))
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
