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
	abuse := readText("docs/abuse.mdx")
	allUsageCapMeters := []string{
		"worker_requests",
		"worker_cpu_ms",
		"worker_transfer_bytes",
		"static_transfer_bytes",
		"sqlite_rows_read",
		"sqlite_rows_written",
		"sqlite_storage_bytes",
		"kv_reads",
		"kv_writes",
		"kv_deletes",
		"kv_lists",
		"kv_storage_bytes",
		"queue_operations",
		"state_requests",
		"state_duration_gb_milliseconds",
		"state_sqlite_rows_read",
		"state_sqlite_rows_written",
		"state_sqlite_storage_bytes",
		"object_storage_bytes",
		"object_storage_class_a_operations",
		"object_storage_class_b_operations",
		"object_storage_egress_bytes",
		"build_minutes",
		"log_events",
	}
	for _, meter := range allUsageCapMeters {
		requireContains(products, "`"+meter+"`", "product docs usage meter "+meter)
		requireContains(limits, "`"+meter+"`", "limits docs usage cap meter "+meter)
		requireContains(openapi, `"`+meter+`"`, "OpenAPI usage meter "+meter)
	}
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

	requireContains(pricing, "`1 GB` per object", "Free State per-object storage docs")
	requireContains(limits, "State SQLite storage is 1 GB per object on Free and 10 GB per object on Pro", "State Free storage limit docs")
	requireContains(limits, "State alarms allow one scheduled alarm per State object", "State alarm limit docs")
	requireContains(limits, "alarm handlers have a `15 minute` wall-clock limit", "State alarm wall-time docs")
	requireContains(limits, "least 30 seconds", "KV minimum expiration docs")
	requireContains(limits, "5,237,760 MiB (`4.995 TiB`) object storage object size ceiling", "object storage max object docs")
	requireContains(limits, "5,115 MiB (`4.995 GiB`) single-part upload limit", "object storage single-part docs")
	requireContains(limits, "automatically switches to multipart upload above", "multipart upload automation docs")
	storage := readText("docs/reference/storage.mdx")
	requireContains(storage, "Direct Multipart API", "multipart API docs")
	requireContains(storage, "5,237,760 MiB (`4.995 TiB`) object storage maximum object size", "object storage provider max docs")
	requireContains(storage, "5,115 MiB (`4.995 GiB`) per single-part object upload", "object storage provider single-part docs")
	requireContains(platform, "Free State objects get 1 GB per object", "platform State Free storage docs")
	requireContains(platform, "Pro State objects get 10 GB per object", "platform State Pro storage docs")
	requireContains(platform, "tovuk state alarm set", "platform State alarm CLI docs")
	requireContains(platform, "TOVUK_RUNTIME_TOKEN=tovuk_runtime_...", "runtime token binding docs")
	requireContains(platform, "TOVUK_SQLITE_DB=sqlite_1", "runtime SQLite binding docs")
	requireContains(platform, "TOVUK_BINDING_AUTH_SERVICE=service_2", "runtime service binding docs")
	requireContains(platform, "It cannot manage account settings", "runtime token scope docs")
	requireContains(pricing, "Pro costs `$5/month`", "pricing Pro subscription docs")
	requireContains(products, "Use `products[].features`", "product choice docs")
	requireContains(products, "`products[].features`", "product feature docs")
	requireContains(products, "`products[].meter_details`", "product meter detail docs")
	requireContains(products, "`products[].pricing_fields`", "product pricing field docs")
	requireContains(products, "Worker:\nUse for Rust public APIs", "Worker product docs")
	requireContains(products, "Features include Rust-only HTTP routes", "Worker feature docs")
	requireContains(products, "State:\nUse for keyed realtime coordination", "State product docs")
	requireContains(products, "Features include State classes", "State feature docs")
	requireContains(products, "Use State alarms for per-object scheduled wake-ups", "State alarm product docs")
	requireContains(products, "`state_duration_gb_milliseconds`", "State meter docs")
	requireContains(products, "runtime, storage, payload, batching", "limit field planning docs")
	requireContains(products, "Keep JavaScript and TypeScript frontend-only", "frontend-only product docs")
	requireContains(pricing, "`usage.billingEstimate`", "pricing billing estimate docs")
	requireContains(pricing, "explicitly free transfer meter", "pricing free transfer estimate docs")
	requireContains(pricing, "`free_transfer`", "pricing free transfer billing unit docs")
	requireContains(products, "`billingEstimate` line items", "product billing estimate docs")
	requireContains(products, "explicit zero-price line item", "product free meter estimate docs")
	requireContains(limits, "`free_transfer`", "limit free transfer estimate docs")
	requireContains(openapi, "build_minutes", "public OpenAPI build minutes meter")
	requireContains(openapi, `"billingEstimate"`, "OpenAPI billing estimate field")
	requireContains(openapi, `"UsageCostEstimate"`, "OpenAPI usage estimate schema")
	requireContains(openapi, `"UsageCostLineItem"`, "OpenAPI usage estimate line schema")
	requireContains(openapi, `"subscriptionUsdCentsPerMonth": 500`, "OpenAPI Pro subscription price")
	requireContains(openapi, `"subscriptionUsdMicrosPerMonth": 5000000`, "OpenAPI Pro subscription estimate")
	requireContains(openapi, `"estimatedMonthlyTotalUsdMicros": 5470000`, "OpenAPI Pro estimate total")
	requireContains(openapi, `"features"`, "OpenAPI product features")
	requireContains(openapi, `"Rust-only HTTP runtime for public APIs under /api/*"`, "OpenAPI worker feature example")
	requireContains(openapi, `"meter_details"`, "OpenAPI product meter details")
	requireContains(openapi, `"ProductMeterEntry"`, "OpenAPI product meter schema")
	requireContains(openapi, `"worker_transfer_bytes"`, "OpenAPI worker transfer meter detail")
	requireContains(openapi, `"object_storage_egress_bytes"`, "OpenAPI object storage egress meter")
	requireContains(openapi, `"pricing_fields"`, "OpenAPI product pricing fields")
	requireContains(openapi, `"pricing.workers.includedRequestsPerMonth"`, "OpenAPI worker pricing field example")
	requireContains(openapi, `"workerDefaultCpuMsPerRequest"`, "OpenAPI worker default CPU limit field")
	requireContains(openapi, `"workerRequestBodySizeMib"`, "OpenAPI worker request body limit field")
	requireContains(openapi, `"workerWebsocketMessageMib"`, "OpenAPI worker websocket limit field")
	requireContains(openapi, `"serviceBindingInvocationsPerRequest": 32`, "OpenAPI service binding invocation limit")
	requireContains(openapi, `"serviceBindingInvocationsPerRequest"`, "OpenAPI service binding invocation field")
	requireContains(openapi, `"buildDeployHooksPerServicePerMinute": 10`, "OpenAPI build service hook rate limit")
	requireContains(openapi, `"buildDeployHooksPerAccountPerMinute": 100`, "OpenAPI build account hook rate limit")
	requireContains(openapi, `"buildCpuMillicores": 2000`, "OpenAPI build CPU millicore limit")
	requireContains(openapi, `"buildEnvVars": 64`, "OpenAPI build env var limit")
	requireContains(openapi, `"buildEnvVarSizeKib": 5`, "OpenAPI build env var size limit")
	requireContains(openapi, `"alwaysOnServices": 500`, "OpenAPI Pro always-on service limit")
	requireContains(openapi, `"logEventSizeBytes": 262144`, "OpenAPI log event size limit")
	requireContains(openapi, `"stateSqliteStorageMibPerObject": 953`, "OpenAPI Free State storage limit")
	requireContains(openapi, `"stateSqliteStorageMibPerObject": 9536`, "OpenAPI Pro State storage limit")
	requireContains(openapi, `"stateAlarmDurationSeconds": 900`, "OpenAPI State alarm wall-time limit")
	requireContains(openapi, `"/v1/services/{service_id}/state/namespaces/{namespace}/objects/{object_key}/alarm"`, "OpenAPI State alarm route")
	requireContains(openapi, `"StateAlarmSetRequest"`, "OpenAPI State alarm request schema")
	requireContains(openapi, `"storageObjectMaxMib": 5237760`, "OpenAPI object size limit")
	requireContains(openapi, `"storageSinglePartUploadMaxMib": 5115`, "OpenAPI single-part upload limit")
	requireContains(openapi, `"storageMultipartUploadMaxMib": 5237760`, "OpenAPI multipart upload size limit")
	requireContains(openapi, `"storageBucketManagementOperationsPerSecond": 50`, "OpenAPI storage bucket management limit")
	requireContains(openapi, `"storageSameKeyWritesPerSecond": 1`, "OpenAPI storage same-key write limit")
	requireContains(openapi, `"objectStorageEgressBytesPerMonth": 1000000000000`, "OpenAPI object storage egress limit")
	requireContains(openapi, `"includedEgressBytesPerMonth": 1000000000000`, "OpenAPI object storage included egress pricing")
	requireContains(openapi, `"egressOverageUsdMicrosPerTb": 1200000`, "OpenAPI object storage egress overage pricing")
	requireContains(openapi, `"/v1/services/{service_id}/storage/multipart/create"`, "OpenAPI multipart create route")
	requireContains(openapi, `"StorageMultipartCreateRequest"`, "OpenAPI multipart create schema")
	requireContains(openapi, `"StorageMultipartCompleteRequest"`, "OpenAPI multipart complete schema")
	requireContains(abuse, "tovuk abuse report", "abuse report CLI docs")
	requireContains(abuse, "tovuk abuse list", "abuse list CLI docs")
	requireContains(abuse, "tovuk abuse appeal", "abuse appeal CLI docs")
	requireContains(abuse, "tovuk abuse quarantine", "abuse quarantine CLI docs")
	requireContains(abuse, "tovuk abuse release", "abuse release CLI docs")
	requireContains(openapi, `"/v1/abuse/reports"`, "OpenAPI abuse reports route")
	requireContains(openapi, `"/v1/abuse/reports/{report_id}/appeal"`, "OpenAPI abuse appeal route")
	requireContains(openapi, `"/v1/operator/abuse/reports/{report_id}/quarantine"`, "OpenAPI abuse quarantine route")
	requireContains(openapi, `"/v1/operator/abuse/reports/{report_id}/release"`, "OpenAPI abuse release route")
	requireContains(openapi, `"AbuseReportCreateRequest"`, "OpenAPI abuse create schema")
	requireContains(openapi, `"AbuseReportCreateResponse"`, "OpenAPI abuse create response schema")
	requireContains(openapi, `"AbuseModerationRequest"`, "OpenAPI abuse moderation request schema")
	requireContains(openapi, `"AbuseModerationResponse"`, "OpenAPI abuse moderation response schema")
	requireContains(openapi, `"AbuseReportsResponse"`, "OpenAPI abuse list response schema")
	requireContains(openapi, `"tovuk abuse list --json"`, "OpenAPI abuse next command")
	requireContains(limits, "tovuk limits set build_minutes", "build minutes cap docs")
	requireContains(limits, "Deploy hooks can trigger 10 builds per service", "build hook rate docs")
	requireContains(limits, "WebSocket response-side tunnel bytes", "worker websocket transfer meter docs")
	requireContains(limits, "Service binding call chains can use up to 32 worker invocations", "service binding invocation limit docs")
	requireContains(readText("docs/reference/workers.mdx"), "response-side tunnel", "worker websocket transfer docs")
	rejectContains(pricing, "account SQLite storage, `10 GB` per object", "stale Free State storage docs")
	rejectContains(limits, "State SQLite storage is 10 GB per object on Free and Pro", "stale State storage docs")
	rejectContains(limits, "least 60 seconds", "stale KV minimum expiration docs")
	rejectContains(limits, "4,768,371 MiB", "stale object storage object docs")
	rejectContains(limits, "4,768 MiB", "stale object storage single-part docs")
	rejectContains(storage, "4,768,371 MiB", "stale object storage object docs")
	rejectContains(storage, "4,768 MiB", "stale object storage single-part docs")
	rejectContains(limits, "when multipart upload support is exposed", "stale multipart docs")
	rejectContains(platform, "Free and Pro State objects get 10 GB per object", "stale platform State storage docs")
	rejectContains(pricing, "$25/month", "stale Pro subscription docs")
	rejectContains(openapi, `"subscriptionUsdCentsPerMonth": 2500`, "stale OpenAPI Pro subscription price")
	rejectContains(openapi, `"subscriptionUsdMicrosPerMonth": 25000000`, "stale OpenAPI Pro subscription estimate")
	rejectContains(openapi, `"estimatedMonthlyTotalUsdMicros": 25470000`, "stale OpenAPI Pro estimate total")
	rejectContains(openapi, `"buildEnvVars": 128`, "stale build env var limit")
	rejectContains(openapi, `"buildCpuMillis"`, "stale build CPU millis limit")
	rejectContains(openapi, `"stateSqliteStorageMib":`, "stale OpenAPI State storage field")
	rejectContains(openapi, `"storageObjectMaxMib": 4768371`, "stale OpenAPI object size limit")
	rejectContains(openapi, `"storageObjectMaxMib": 5242880`, "stale OpenAPI object size limit")
	rejectContains(openapi, `"storageSinglePartUploadMaxMib": 4768`, "stale OpenAPI single-part upload limit")
	rejectContains(openapi, `"storageSinglePartUploadMaxMib": 5120`, "stale OpenAPI single-part upload limit")
	rejectContains(storage, "free egress", "stale object storage egress docs")
	rejectContains(pricing, "free egress", "stale pricing object storage egress docs")
	rejectContains(openapi, "tovuk caps", "retired usage caps command in OpenAPI")

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
