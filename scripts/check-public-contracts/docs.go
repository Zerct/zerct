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

	navPages := strings.Join(pages, "\n")
	openapi := readText("docs/openapi.json")
	readme := readText("README.md")
	deploy := readText("docs/deploy.mdx")
	pricing := readText("docs/pricing.mdx")
	limits := readText("docs/reference/limits.mdx")
	resources := readText("docs/reference/resources.mdx")
	products := readText("docs/reference/products.mdx")
	packages := readText("docs/reference/packages.mdx")
	kv := readText("docs/reference/kv.mdx")
	sqlite := readText("docs/reference/sqlite.mdx")
	state := readText("docs/reference/state.mdx")
	queues := readText("docs/reference/queues.mdx")
	cron := readText("docs/reference/cron.mdx")
	bindings := readText("docs/reference/bindings.mdx")
	secrets := readText("docs/reference/secrets.mdx")
	domains := readText("docs/reference/domains.mdx")
	logsBuilds := readText("docs/reference/logs-builds.mdx")
	usageCaps := readText("docs/reference/usage-caps.mdx")
	llms := readText("docs/llms.txt")
	abuse := readText("docs/abuse.mdx")
	support := readText("docs/support.mdx")
	agents := readText("docs/agents.mdx")
	publicCopy := strings.Join([]string{
		readme,
		openapi,
		readText("docs/docs.json"),
		readText("docs/llms.txt"),
		readText("docs/skill.md"),
		strings.Join(readSortedTextsRecursive("docs", ".mdx"), "\n"),
		readText("crates/tovuk/README.md"),
		readText("packages/tovuk/README.md"),
		readText("packages/tovuk/package.json"),
		readText("packages/tovuk-py/README.md"),
		readText("packages/tovuk-py/pyproject.toml"),
		readText("skills/tovuk/SKILL.md"),
	}, "\n")
	rejectForbiddenPublicCopyTerms("public docs and package copy", publicCopy)
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
	for _, page := range []string{
		"reference/kv",
		"reference/queues",
		"reference/cron",
		"reference/bindings",
		"reference/secrets",
		"reference/domains",
		"reference/logs-builds",
		"reference/usage-caps",
	} {
		requireContains(navPages, page, "Mintlify resource navigation "+page)
		requireContains(llms, "docs/"+page+".mdx", "llms resource reference "+page)
	}
	checkOpenAPIMeterContracts("docs/openapi.json", allUsageCapMeters)
	retiredFullstackKind := "worker" + "_static"
	retiredFullstackTemplate := "worker" + "-static-rust-tanstack"
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
		"wants_database",
		"wantsDatabase",
		"/v1/services/{service_id}/database",
		"DatabaseResponse",
		"database_url_injected",
		retiredFullstackKind,
		retiredFullstackTemplate,
	} {
		rejectContains(openapi, retired, "retired public app contract")
	}
	for _, retired := range []string{
		"primary app URL",
		"Customer Apps",
		"app terms",
		"app data",
		"my-app",
		"name = \"dashboard\"",
	} {
		rejectContains(publicCopy, retired, "retired service wording")
	}

	requireContains(pricing, "`1 GB` per object", "Free State per-object storage docs")
	requireContains(pricing, "`5 GB` included storage per month", "Pro State included storage docs")
	requireContains(pricing, "`10 GB`", "Pro State hard cap value docs")
	requireContains(pricing, "per-object hard cap", "Pro State hard cap docs")
	rejectContains(pricing, "`5 GB` storage per month", "ambiguous Pro State storage pricing docs")
	requireContains(limits, "compiled artifacts such as `.exe`, `.so`,", "compiled deploy artifact docs")
	requireContains(deploy, "compiled artifacts such as `.exe`, `.so`,", "compiled deploy artifact guide")
	requireContains(readme, "compiled artifacts such as `.exe`, `.so`,", "compiled deploy artifact README")
	requireContains(agents, "compiled artifacts such as `.exe`, `.so`,", "compiled deploy artifact agent docs")
	requireContains(agents, "Dashboard Overview is available", "dashboard overview agent docs")
	requireContains(agents, "same first\nService command sequence", "dashboard first Service command sequence docs")
	requireContains(agents, "tovuk new hello-service --template fullstack-rust-tanstack", "dashboard first Service template docs")
	requireContains(agents, "committed `tovuk.toml` remains\nthe source of truth", "dashboard source of truth docs")
	requireContains(agents, "`tovuk service delete <service> --json` commands", "dashboard service delete docs")
	requireContains(agents, "`DELETE /v1/services/{service_id}`", "dashboard service delete API docs")
	requireContains(llms, "Dashboard Overview is available", "llms dashboard overview guidance")
	requireContains(llms, "shows first Service\ncommands", "llms dashboard first Service guidance")
	requireContains(llms, "committed `tovuk.toml` remains the source of\ntruth", "llms dashboard source of truth guidance")
	requireContains(llms, "`tovuk service delete <service> --json`", "llms dashboard service delete guidance")
	requireContains(llms, "`DELETE /v1/services/{service_id}`", "llms dashboard service delete API guidance")
	requireContains(packages, "billing, support, and abuse", "packages abuse command surface docs")
	requireContains(packages, "storage, SQLite, KV,\nqueues, cron, State, service bindings", "packages resource command surface docs")
	requireContains(limits, "State SQLite storage is 1 GB per object on Free and 10 GB per object on Pro", "State Free storage limit docs")
	requireContains(limits, "State alarms allow one scheduled alarm per State object", "State alarm limit docs")
	requireContains(limits, "alarm handlers have a `15 minute` wall-clock limit", "State alarm wall-time docs")
	requireContains(limits, "least 30 seconds", "KV minimum expiration docs")
	requireContains(limits, "5,237,760 MiB (`4.995 TiB`) object storage object size ceiling", "object storage max object docs")
	requireContains(limits, "5,115 MiB (`4.995 GiB`) single-part upload limit", "object storage single-part docs")
	requireContains(limits, "automatically switches to multipart upload above", "multipart upload automation docs")
	requireContains(limits, "Public media uploads are scanned", "public media scanning limit docs")
	storage := readText("docs/reference/storage.mdx")
	requireContains(storage, "Direct Multipart API", "multipart API docs")
	requireContains(storage, "5,237,760 MiB (`4.995 TiB`) object storage maximum object size", "object storage provider max docs")
	requireContains(storage, "5,115 MiB (`4.995 GiB`) per single-part object upload", "object storage provider single-part docs")
	requireContains(storage, "Public media uploads are scanned", "public media scanning docs")
	requireContains(storage, "https://tovuk.com/account/storage", "dashboard storage route docs")
	requireContains(storage, "https://tovuk.com/<handle>/storage", "dashboard storage handle route docs")
	requireContains(storage, "`GET /v1/services/{service_id}/storage`", "dashboard storage list API docs")
	requireContains(storage, "`POST /v1/services/{service_id}/storage/upload-url`", "dashboard storage upload API docs")
	requireContains(storage, "`POST /v1/services/{service_id}/storage/multipart/create`", "dashboard storage multipart create API docs")
	requireContains(storage, "`POST /v1/services/{service_id}/storage/multipart/complete`", "dashboard storage multipart complete API docs")
	requireContains(storage, "`POST /v1/services/{service_id}/storage/multipart/abort`", "dashboard storage multipart abort API docs")
	requireContains(storage, "`DELETE /v1/services/{service_id}/storage?path=<path>`", "dashboard storage delete API docs")
	requireContains(resources, "Free State objects get 1 GB per object", "resources State Free storage docs")
	requireContains(resources, "Pro State objects get 10 GB per object", "resources State Pro storage docs")
	requireContains(resources, "tovuk state alarm set", "resources State alarm CLI docs")
	requireContains(resources, "TOVUK_RUNTIME_TOKEN=tovuk_runtime_...", "runtime token binding docs")
	requireContains(resources, "TOVUK_SQLITE_DB=sqlite_1", "runtime SQLite binding docs")
	requireContains(resources, "TOVUK_BINDING_AUTH_SERVICE=service_2", "runtime service binding docs")
	requireContains(resources, "It cannot manage account settings", "runtime token scope docs")
	requireContains(pricing, "Pro costs `$5/month`", "pricing Pro subscription docs")
	requireContains(products, "Use `products[].features`", "product choice docs")
	requireContains(products, "`products[].features`", "product feature docs")
	requireContains(products, "`products[].meter_details`", "product meter detail docs")
	requireContains(products, "`cap_commands`", "product meter cap command docs")
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
	requireContains(openapi, `"/v1/account"`, "OpenAPI account profile route")
	requireContains(openapi, `"/v1/account/overview"`, "OpenAPI account overview route")
	requireContains(openapi, `"/v1/account/activity"`, "OpenAPI account activity route")
	requireContains(openapi, `"AccountOverviewResponse"`, "OpenAPI account overview response schema")
	requireContains(openapi, `"AccountOverviewService"`, "OpenAPI account overview service schema")
	requireContains(openapi, "Explicit enabled and disabled service capabilities from tovuk.toml", "OpenAPI account overview service capabilities")
	requireContains(openapi, `"AccountOverviewServiceResources"`, "OpenAPI account overview service resources schema")
	requireContains(openapi, `"sqliteDatabases"`, "OpenAPI account overview service SQLite resource count")
	requireContains(openapi, `"serviceBindings"`, "OpenAPI account overview service binding resource count")
	requireContains(openapi, `"AccountOverviewUsage"`, "OpenAPI account overview usage schema")
	requireContains(openapi, `"billingEstimate"`, "OpenAPI billing estimate field")
	requireContains(openapi, `"accountUsage"`, "OpenAPI service overview account usage field")
	requireContains(openapi, "Explicit service capabilities from tovuk.toml", "OpenAPI service capabilities")
	requireContains(openapi, "account usage, billing estimate, and next actions", "OpenAPI service overview billing description")
	requireContains(resources, "`accountUsage`, `billingEstimate`", "service show account usage billing docs")
	requireContains(resources, "resources, capabilities, `accountUsage`, `billingEstimate`", "service show capabilities docs")
	requireContains(agents, "per-Service resource counts for SQLite", "agents dashboard overview resource count docs")
	requireContains(agents, "enabled and disabled capability summaries", "agents dashboard overview capability docs")
	requireContains(agents, "`tovuk logs --service <service> --limit 100 --json`", "agents dashboard overview logs command docs")
	requireContains(agents, "`tovuk storage list --service <service> --json`", "agents dashboard overview storage command docs")
	requireContains(agents, "`tovuk limits set worker_requests --period day --value 100000 --notify-at-percent 80 --json`", "agents dashboard overview usage cap command docs")
	requireContains(agents, "`tovuk support create \"Service issue\" ... --service <service> --json`", "agents dashboard overview support command docs")
	requireContains(llms, "Dashboard Overview is available", "llms dashboard overview docs")
	requireContains(llms, "Service kind, per-Service resource counts", "llms dashboard resource count docs")
	requireContains(llms, "enabled and disabled capability summaries", "llms dashboard capability docs")
	requireContains(llms, "`tovuk logs --service <service> --limit 100 --json`", "llms dashboard overview logs command docs")
	requireContains(llms, "`tovuk storage list --service <service> --json`", "llms dashboard overview storage command docs")
	requireContains(llms, "`tovuk limits set worker_requests --period day --value 100000 --notify-at-percent 80 --json`", "llms dashboard overview usage cap command docs")
	requireContains(llms, "`tovuk support create \"Service issue\" ... --service <service> --json`", "llms dashboard overview support command docs")
	requireContains(resources, "https://tovuk.com/account/resources", "dashboard resources route docs")
	requireContains(resources, "same API routes and limits as the CLI", "dashboard resources parity docs")
	requireContains(resources, "creating and deleting SQLite", "dashboard resources create delete docs")
	requireContains(resources, "Binding resources through the same API routes as the CLI", "dashboard resources delete parity docs")
	requireContains(resources, "single SQLite queries", "dashboard SQLite query resources docs")
	requireContains(resources, "list SQLite backups", "dashboard SQLite backup resources docs")
	requireContains(resources, "restore SQLite backups", "dashboard SQLite restore resources docs")
	requireContains(resources, "`POST /v1/services/{service_id}/sqlite/{database}/query`", "dashboard SQLite query API docs")
	requireContains(resources, "`vmSteps`, `fullscanSteps`", "dashboard SQLite query metering docs")
	requireContains(resources, "list KV keys, read one KV key", "dashboard KV guidance")
	requireContains(resources, "Use CLI bulk commands", "dashboard KV bulk guidance")
	requireContains(resources, "read Queue messages", "dashboard Queue messages guidance")
	requireContains(resources, "Queue metrics", "dashboard Queue metrics guidance")
	requireContains(resources, "send one Queue message", "dashboard Queue send guidance")
	requireContains(resources, "send one Queue batch", "dashboard Queue batch guidance")
	requireContains(resources, "update Queue retries", "dashboard Queue update guidance")
	requireContains(resources, "`PUT /v1/services/{service_id}/queues/{queue}`", "dashboard Queue update API docs")
	requireContains(resources, "update Cron schedules", "dashboard Cron update guidance")
	requireContains(resources, "pause Cron triggers", "dashboard Cron pause guidance")
	requireContains(resources, "resume", "dashboard Cron resume guidance")
	requireContains(resources, "State objects", "dashboard State object guidance")
	requireContains(resources, "State keys", "dashboard State key guidance")
	requireContains(resources, "read State alarms, set State alarms", "dashboard State alarm guidance")
	requireContains(sqlite, "https://tovuk.com/account/resources", "SQLite dashboard route docs")
	requireContains(sqlite, "`POST /v1/services/{service_id}/sqlite/{database}/query`", "SQLite dashboard query API docs")
	requireContains(sqlite, "`GET /v1/services/{service_id}/sqlite/{database}/backups`", "SQLite dashboard backup list API docs")
	requireContains(sqlite, "`POST /v1/services/{service_id}/sqlite/{database}/backups`", "SQLite dashboard backup create API docs")
	requireContains(sqlite, "`POST /v1/services/{service_id}/sqlite/{database}/backups/{backup}/restore`", "SQLite dashboard backup restore API docs")
	requireContains(sqlite, "`vmSteps`, `fullscanSteps`", "SQLite dashboard metering docs")
	requireContains(sqlite, "Use the CLI for batch statements", "SQLite dashboard batch guidance")
	requireContains(state, "https://tovuk.com/account/resources", "State dashboard route docs")
	requireContains(state, "list State objects", "State dashboard object docs")
	requireContains(state, "`GET /v1/services/{service_id}/state/namespaces/{namespace}/objects`", "State dashboard objects API docs")
	requireContains(state, "`PUT /v1/services/{service_id}/state/namespaces/{namespace}/objects/{object_key}/values/{key}`", "State dashboard value write API docs")
	requireContains(state, "`DELETE /v1/services/{service_id}/state/namespaces/{namespace}/objects/{object_key}/alarm`", "State dashboard alarm delete API docs")
	requireContains(llms, "Dashboard Resources is available", "llms dashboard resources guidance")
	requireContains(llms, "creating and deleting SQLite", "llms dashboard resources create delete guidance")
	requireContains(llms, "Binding resources through the same API routes as the CLI", "llms dashboard resources delete parity guidance")
	requireContains(llms, "single SQLite queries", "llms dashboard SQLite query guidance")
	requireContains(llms, "create SQLite backups", "llms dashboard SQLite backup create guidance")
	requireContains(llms, "restore SQLite backups", "llms dashboard SQLite backup restore guidance")
	requireContains(llms, "`vmSteps`, `fullscanSteps`", "llms dashboard SQLite metering guidance")
	requireContains(llms, "list KV", "llms dashboard KV guidance")
	requireContains(llms, "`kv_reads`, `kv_writes`", "llms dashboard KV metering guidance")
	requireContains(llms, "update Queue retries", "llms dashboard Queue update guidance")
	requireContains(llms, "read Queue messages, read Queue metrics", "llms dashboard Queue guidance")
	requireContains(llms, "`queue_operations` metering", "llms dashboard Queue metering guidance")
	requireContains(llms, "update Cron schedules", "llms dashboard Cron update guidance")
	requireContains(llms, "pause Cron triggers", "llms dashboard Cron pause guidance")
	requireContains(llms, "resume Cron triggers", "llms dashboard Cron resume guidance")
	requireContains(llms, "list State objects", "llms dashboard State object guidance")
	requireContains(llms, "read State values", "llms dashboard State value guidance")
	requireContains(llms, "State alarms", "llms dashboard State alarm guidance")
	requireContains(llms, "`state_requests`, `state_duration_gb_milliseconds`", "llms dashboard State metering guidance")
	requireContains(bindings, "/v1/services/{service_id}/service-bindings", "service binding API docs")
	requireContains(openapi, `"UsageCostEstimate"`, "OpenAPI usage estimate schema")
	requireContains(openapi, `"UsageCostLineItem"`, "OpenAPI usage estimate line schema")
	requireContains(openapi, `"subscriptionUsdCentsPerMonth": 500`, "OpenAPI Pro subscription price")
	requireContains(openapi, `"subscriptionUsdMicrosPerMonth": 5000000`, "OpenAPI Pro subscription estimate")
	requireContains(openapi, `"estimatedMonthlyTotalUsdMicros": 5470000`, "OpenAPI Pro estimate total")
	requireContains(openapi, `"features"`, "OpenAPI product features")
	requireContains(openapi, `"Rust-only HTTP runtime for public APIs under /api/*"`, "OpenAPI worker feature example")
	requireContains(openapi, `"meter_details"`, "OpenAPI product meter details")
	requireContains(openapi, `"ProductMeterEntry"`, "OpenAPI product meter schema")
	requireContains(openapi, `"cap_commands"`, "OpenAPI product meter cap commands")
	requireContains(openapi, `"ProductMeterCapCommands"`, "OpenAPI product meter cap command schema")
	requireContains(openapi, `"tovuk limits set worker_requests --period day --value <value> --notify-at-percent 80 --json"`, "OpenAPI meter daily cap command")
	requireContains(openapi, `"worker_transfer_bytes"`, "OpenAPI worker transfer meter detail")
	requireContains(openapi, `"object_storage_class_a_operations"`, "OpenAPI object storage Class A meter")
	requireContains(openapi, `"object_storage_class_b_operations"`, "OpenAPI object storage Class B meter")
	requireContains(openapi, `"classAOverageUsdMicrosPerMillion": 4500000`, "OpenAPI object storage Class A pricing example")
	requireContains(openapi, `"classBOverageUsdMicrosPerMillion": 360000`, "OpenAPI object storage Class B pricing example")
	requireContains(products, "pricing.objectStorage.classAOverageUsdMicrosPerMillion", "product docs object storage Class A pricing field")
	requireContains(products, "pricing.objectStorage.classBOverageUsdMicrosPerMillion", "product docs object storage Class B pricing field")
	requireContains(limits, "tovuk limits set object_storage_class_a_operations --period month --value 1000000 --notify-at-percent 80 --json", "limit docs object storage Class A cap command")
	requireContains(limits, "tovuk limits set object_storage_class_b_operations --period month --value 10000000 --notify-at-percent 80 --json", "limit docs object storage Class B cap command")
	requireContains(usageCaps, "https://tovuk.com/account/usage", "dashboard usage caps route docs")
	requireContains(usageCaps, "same API routes and rate limits as the CLI", "dashboard usage caps parity docs")
	requireContains(usageCaps, "copyable `tovuk limits set` commands with `--notify-at-percent`", "dashboard usage cap notification command docs")
	requireContains(usageCaps, "Pass `--notify-at-percent <1-100>`", "usage cap notification threshold docs")
	requireContains(usageCaps, "hosted checkout flow as `tovuk billing checkout --json`", "dashboard usage checkout docs")
	requireContains(usageCaps, "billing portal flow as `tovuk billing portal`", "dashboard usage billing portal docs")
	requireContains(llms, "Dashboard Usage is available", "llms dashboard usage guidance")
	requireContains(llms, "`tovuk limits set` commands with `--notify-at-percent`", "llms dashboard usage cap notification command guidance")
	requireContains(llms, "checkout flow as `tovuk billing checkout --json`", "llms dashboard usage checkout guidance")
	requireContains(llms, "billing portal flow as", "llms dashboard usage billing portal guidance")
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
	requireContains(support, "https://tovuk.com/account/support", "dashboard support route docs")
	requireContains(support, "same API routes and rate limits as the CLI", "dashboard support parity docs")
	requireContains(support, "copy `tovuk support list --json`", "dashboard support copy command docs")
	requireContains(support, "`tovuk support resolve <ticket_id> --json`", "dashboard support resolve command docs")
	requireContains(support, "`tovuk logs --build <build_id> --json`", "dashboard support build logs command docs")
	requireContains(llms, "Dashboard Support is available", "llms dashboard support guidance")
	requireContains(llms, "Ticket rows expose copyable", "llms dashboard support row command guidance")
	requireContains(abuse, "https://tovuk.com/account/abuse", "dashboard abuse route docs")
	requireContains(abuse, "same API routes and rate limits as the CLI", "dashboard abuse parity docs")
	requireContains(abuse, "Report rows expose copyable `tovuk abuse list --json`", "dashboard abuse copy command docs")
	requireContains(abuse, "`tovuk abuse appeal <report_id> \"Remediation details\" --json`", "dashboard abuse appeal command docs")
	requireContains(abuse, "tovuk service show service_1 --json", "abuse service context docs")
	requireContains(llms, "Dashboard Abuse is available", "llms dashboard abuse guidance")
	requireContains(llms, "Report rows expose copyable `tovuk abuse list --json`", "llms dashboard abuse row command guidance")
	requireContains(llms, "before service-scoped reports", "llms abuse service context guidance")
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
	requireContains(openapi, "Public media rejects executable and script payloads", "OpenAPI public media scanning policy")
	requireContains(openapi, "compiled artifacts are rejected", "OpenAPI deploy archive artifact policy")
	requireContains(abuse, "tovuk abuse report", "abuse report CLI docs")
	requireContains(abuse, "tovuk abuse list", "abuse list CLI docs")
	requireContains(abuse, "tovuk abuse list --operator", "operator abuse list CLI docs")
	requireContains(abuse, "tovuk abuse appeal", "abuse appeal CLI docs")
	requireContains(abuse, "tovuk abuse triage", "abuse triage CLI docs")
	requireContains(abuse, "tovuk abuse notify-owner", "abuse owner notification CLI docs")
	requireContains(abuse, "tovuk abuse quarantine", "abuse quarantine CLI docs")
	requireContains(abuse, "tovuk abuse resolve", "abuse resolve CLI docs")
	requireContains(abuse, "tovuk abuse reject", "abuse reject CLI docs")
	requireContains(abuse, "tovuk abuse release", "abuse release CLI docs")
	requireContains(openapi, `"/v1/abuse/reports"`, "OpenAPI abuse reports route")
	requireContains(openapi, `"/v1/abuse/reports/{report_id}/appeal"`, "OpenAPI abuse appeal route")
	requireContains(openapi, `"/v1/operator/abuse/reports"`, "OpenAPI operator abuse reports route")
	requireContains(openapi, `"/v1/operator/abuse/reports/{report_id}/quarantine"`, "OpenAPI abuse quarantine route")
	requireContains(openapi, `"/v1/operator/abuse/reports/{report_id}/triage"`, "OpenAPI abuse triage route")
	requireContains(openapi, `"/v1/operator/abuse/reports/{report_id}/notify-owner"`, "OpenAPI abuse owner notification route")
	requireContains(openapi, `"/v1/operator/abuse/reports/{report_id}/resolve"`, "OpenAPI abuse resolve route")
	requireContains(openapi, `"/v1/operator/abuse/reports/{report_id}/reject"`, "OpenAPI abuse reject route")
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
	requireContains(kv, "tovuk kv create --service service_1 CACHE --json", "KV create docs")
	requireContains(kv, "`kv_reads`, `kv_writes`, `kv_deletes`, `kv_lists`, and", "KV meter docs")
	requireContains(kv, "https://tovuk.com/account/resources", "KV dashboard route docs")
	requireContains(kv, "`GET /v1/services/{service_id}/kv/{namespace}/keys`", "KV key list API docs")
	requireContains(kv, "`GET /v1/services/{service_id}/kv/{namespace}/values/{key}`", "KV key read API docs")
	requireContains(kv, "`PUT /v1/services/{service_id}/kv/{namespace}/values/{key}`", "KV key write API docs")
	requireContains(kv, "`DELETE /v1/services/{service_id}/kv/{namespace}/values/{key}`", "KV key delete API docs")
	requireContains(kv, "Use the CLI or API bulk routes for bulk reads", "KV dashboard bulk guidance")
	rejectContains(kv, "/kv/namespaces/{namespace}/values", "stale KV key value API path docs")
	rejectContains(kv, "/kv/namespaces/{namespace}/keys", "stale KV key list API path docs")
	requireContains(kv, "Bulk reads accept 100 keys", "KV bulk limit docs")
	requireContains(queues, "tovuk queue send --service service_1 jobs", "queue send docs")
	requireContains(queues, "tovuk queue send-batch --service service_1 jobs", "queue batch send docs")
	requireContains(queues, "https://tovuk.com/account/resources", "Queue dashboard route docs")
	requireContains(queues, "update Queue retries", "Queue dashboard update docs")
	requireContains(queues, "`GET /v1/services/{service_id}/queues/{queue}/messages`", "Queue messages API docs")
	requireContains(queues, "`POST /v1/services/{service_id}/queues/{queue}/messages/batch`", "Queue batch API docs")
	requireContains(queues, "`PUT /v1/services/{service_id}/queues/{queue}`", "Queue update API docs")
	requireContains(queues, "dead-letter queue", "queue dead-letter docs")
	requireContains(queues, "`queue_operations`", "queue meter docs")
	requireContains(cron, "tovuk cron create --service service_1 nightly", "cron create docs")
	requireContains(cron, "tovuk cron update --service service_1 nightly", "cron update docs")
	requireContains(cron, "tovuk cron disable --service service_1 nightly", "cron disable docs")
	requireContains(cron, "tovuk cron enable --service service_1 nightly", "cron enable docs")
	requireContains(cron, "https://tovuk.com/account/resources", "Cron dashboard route docs")
	requireContains(cron, "`POST /v1/services/{service_id}/cron`", "Cron create API docs")
	requireContains(cron, "`PUT /v1/services/{service_id}/cron/{trigger}`", "Cron update API docs")
	requireContains(cron, "`DELETE /v1/services/{service_id}/cron/{trigger}`", "Cron delete API docs")
	rejectContains(cron, "/cron/triggers", "stale Cron trigger API path docs")
	requireContains(cron, "POST /.tovuk/cron/<trigger>", "cron delivery docs")
	requireContains(bindings, "tovuk binding create --service service_1 AUTH_SERVICE", "service binding create docs")
	requireContains(bindings, "32 worker invocations", "service binding chain docs")
	requireContains(secrets, "API_KEY=\"$API_KEY\"", "secret set docs")
	requireContains(secrets, "`PUT /v1/services/{service_id}/env`", "secret set API docs")
	requireContains(secrets, "Secret values are write-only", "secret write-only docs")
	requireContains(secrets, "https://tovuk.com/account/secrets", "dashboard secrets route docs")
	requireContains(secrets, "same API routes and rate limits as the CLI", "dashboard secrets parity docs")
	requireContains(llms, "Dashboard Secrets is available", "llms dashboard secrets guidance")
	requireContains(llms, "Dashboard Storage is available", "llms dashboard storage guidance")
	requireContains(llms, "automatically uses multipart transfer above 100 MiB", "llms dashboard storage multipart guidance")
	requireContains(domains, "tovuk domains verify --service service_1 api.example.com --json", "domain verify docs")
	requireContains(domains, "Never point an A record at Tovuk origin hosts", "domain safety docs")
	requireContains(logsBuilds, "tovuk deploy cancel deploy_1 --json", "deploy cancel docs")
	requireContains(logsBuilds, "`build_minutes`. Logs use `log_events`", "logs builds meter docs")
	requireContains(logsBuilds, "https://tovuk.com/account/activity", "dashboard activity route docs")
	requireContains(logsBuilds, "same activity,\ndeploy cancel, and log API routes", "dashboard activity API parity docs")
	requireContains(logsBuilds, "Queued deploy rows can cancel queued build work", "dashboard activity cancel action docs")
	requireContains(llms, "Dashboard Activity is available", "llms dashboard activity guidance")
	requireContains(llms, "Queued deploy rows can cancel queued build work", "llms dashboard activity cancel guidance")
	requireContains(usageCaps, "tovuk limits set object_storage_egress_bytes", "usage caps object egress docs")
	requireContains(usageCaps, "`object_storage_class_a_operations`", "usage caps Class A meter docs")
	rejectContains(pricing, "account SQLite storage, `10 GB` per object", "stale Free State storage docs")
	rejectContains(limits, "State SQLite storage is 10 GB per object on Free and Pro", "stale State storage docs")
	rejectContains(limits, "least 60 seconds", "stale KV minimum expiration docs")
	rejectContains(limits, "4,768,371 MiB", "stale object storage object docs")
	rejectContains(limits, "4,768 MiB", "stale object storage single-part docs")
	rejectContains(storage, "4,768,371 MiB", "stale object storage object docs")
	rejectContains(storage, "4,768 MiB", "stale object storage single-part docs")
	rejectContains(limits, "when multipart upload support is exposed", "stale multipart docs")
	rejectContains(resources, "Free and Pro State objects get 10 GB per object", "stale resources State storage docs")
	rejectContains(pricing, "$25/month", "stale Pro subscription docs")
	rejectContains(openapi, `"subscriptionUsdCentsPerMonth": 2500`, "stale OpenAPI Pro subscription price")
	rejectContains(openapi, `"subscriptionUsdMicrosPerMonth": 25000000`, "stale OpenAPI Pro subscription estimate")
	rejectContains(openapi, `"estimatedMonthlyTotalUsdMicros": 25470000`, "stale OpenAPI Pro estimate total")
	rejectContains(openapi, `"buildEnvVars": 128`, "stale build env var limit")
	rejectContains(secrets, "`PUT /v1/services/{service_id}/env/{name}`", "stale secret set API docs")
	rejectContains(openapi, `"/v1/me"`, "retired account profile route")
	rejectContains(openapi, `"/v1/activity"`, "retired account activity route")
	rejectContains(openapi, `"/v1/account/dashboard"`, "retired account dashboard route")
	rejectContains(openapi, `"AccountDashboardResponse"`, "retired account dashboard response schema")
	rejectContains(openapi, `"AccountDashboardService"`, "retired account dashboard service schema")
	rejectContains(openapi, `"AccountDashboardUsage"`, "retired account dashboard usage schema")
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

func checkOpenAPIMeterContracts(openapiPath string, expectedMeters []string) {
	var openapi map[string]interface{}
	readJSON(openapiPath, &openapi)

	accountUsageMeterProperties := schemaProperties(
		openapi,
		"AccountUsageMeterWindow",
		"OpenAPI AccountUsageMeterWindow",
	)
	requireStringSliceExactly(
		interfaceMapKeys(accountUsageMeterProperties),
		expectedMeters,
		"OpenAPI AccountUsageMeterWindow meter properties",
	)

	for _, schema := range []string{"UsageCap", "UsageCapDeleteResponse"} {
		pattern := schemaPropertyPattern(openapi, schema, "metric")
		requireStringSliceExactly(
			meterPatternValues(pattern, schema),
			expectedMeters,
			"OpenAPI "+schema+" metric pattern",
		)
	}

	for _, route := range []struct {
		path   string
		method string
	}{
		{path: "/v1/usage/caps/{metric}", method: "put"},
		{path: "/v1/usage/caps/{metric}/{period}", method: "delete"},
	} {
		pattern := operationParameterPattern(openapi, route.path, route.method, "metric")
		requireStringSliceExactly(
			meterPatternValues(pattern, route.path+" "+route.method),
			expectedMeters,
			"OpenAPI "+route.path+" "+route.method+" metric parameter pattern",
		)
	}
}

func schemaProperties(
	openapi map[string]interface{},
	schemaName string,
	label string,
) map[string]interface{} {
	schema := openAPISchema(openapi, schemaName)
	return objectField(schema, "properties", label+" properties")
}

func schemaPropertyPattern(openapi map[string]interface{}, schemaName string, propertyName string) string {
	properties := schemaProperties(openapi, schemaName, "OpenAPI "+schemaName)
	property := objectField(properties, propertyName, "OpenAPI "+schemaName+"."+propertyName)
	return stringField(property, "pattern", "OpenAPI "+schemaName+"."+propertyName+" pattern")
}

func operationParameterPattern(
	openapi map[string]interface{},
	path string,
	method string,
	parameterName string,
) string {
	paths := objectField(openapi, "paths", "OpenAPI paths")
	pathItem := objectField(paths, path, "OpenAPI path "+path)
	operation := objectField(pathItem, method, "OpenAPI operation "+path+" "+method)
	parameters := arrayField(operation, "parameters", "OpenAPI parameters "+path+" "+method)
	for _, rawParameter := range parameters {
		parameter := objectValue(rawParameter, "OpenAPI parameter "+path+" "+method)
		name := stringField(parameter, "name", "OpenAPI parameter name")
		if name != parameterName {
			continue
		}
		schema := objectField(parameter, "schema", "OpenAPI parameter schema "+parameterName)
		return stringField(schema, "pattern", "OpenAPI parameter pattern "+parameterName)
	}
	fail("OpenAPI %s %s missing parameter %s", path, method, parameterName)
	return ""
}

func openAPISchema(openapi map[string]interface{}, schemaName string) map[string]interface{} {
	components := objectField(openapi, "components", "OpenAPI components")
	schemas := objectField(components, "schemas", "OpenAPI schemas")
	return objectField(schemas, schemaName, "OpenAPI schema "+schemaName)
}

func meterPatternValues(pattern string, label string) []string {
	if !strings.HasPrefix(pattern, "^(") || !strings.HasSuffix(pattern, ")$") {
		fail("%s metric pattern must use ^(...)$ shape", label)
	}
	body := strings.TrimSuffix(strings.TrimPrefix(pattern, "^("), ")$")
	if body == "" {
		fail("%s metric pattern must not be empty", label)
	}
	return strings.Split(body, "|")
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
