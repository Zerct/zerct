import { existsSync, readFileSync } from "node:fs";

const docsConfigPath = "docs/docs.json";
const docsConfig = JSON.parse(readFileSync(docsConfigPath, "utf8"));

const pages = [];

function collectPageEntry(entry) {
  if (typeof entry === "string") {
    pages.push(entry);
    return;
  }

  if (entry && typeof entry === "object" && Array.isArray(entry.pages)) {
    for (const nestedEntry of entry.pages) {
      collectPageEntry(nestedEntry);
    }
  }
}

for (const tab of docsConfig.navigation?.tabs ?? []) {
  for (const group of tab.groups ?? []) {
    for (const page of group.pages ?? []) {
      collectPageEntry(page);
    }
  }
}

const missingPages = pages
  .filter((page) => !page.startsWith("http://") && !page.startsWith("https://"))
  .map((page) => `docs/${page}.mdx`)
  .filter((pagePath) => !existsSync(pagePath));

if (missingPages.length > 0) {
  console.error(`Missing Mintlify pages:\n${missingPages.join("\n")}`);
  process.exit(1);
}

console.log(`Checked ${pages.length} Mintlify navigation entries.`);
