const target = process.argv[2] ?? "https://docs.zerct.com";
const normalizedTarget = target.startsWith("http") ? target : `https://${target}`;
const baseUrl = normalizedTarget.replace(/\/$/, "");

const requiredPaths = [
  "/llms.txt",
  "/llms-full.txt",
  "/skill.md",
  "/.well-known/skills/index.json",
  "/.well-known/agent-skills/index.json",
  "/.well-known/mcp",
  "/sitemap.xml",
  "/robots.txt",
  "/openapi.json"
];

async function fetchText(path, headers = {}) {
  const response = await fetch(`${baseUrl}${path}`, { headers });
  const text = await response.text();

  if (!response.ok) {
    throw new Error(`${path} returned ${response.status}`);
  }

  return { response, text };
}

function assertIncludes(name, text, pattern) {
  if (!pattern.test(text)) {
    throw new Error(`${name} did not match ${pattern}`);
  }
}

for (const path of requiredPaths) {
  const { text } = await fetchText(path);

  if (text.trim().length === 0) {
    throw new Error(`${path} is empty`);
  }
}

const llms = await fetchText("/llms.txt");
assertIncludes("llms.txt", llms.text, /^# /m);
assertIncludes("llms.txt", llms.text, /\[[^\]]+\]\([^)]+\)/);

const skill = await fetchText("/skill.md");
assertIncludes("skill.md", skill.text, /^---\n/m);
assertIncludes("skill.md", skill.text, /name:\s*zerct/i);

const robots = await fetchText("/robots.txt");
if (/Disallow:\s*\//i.test(robots.text) && !/Allow:\s*\//i.test(robots.text)) {
  throw new Error("robots.txt appears to block crawlers");
}

const markdown = await fetchText("/", { Accept: "text/markdown" });
assertIncludes("Markdown content negotiation", markdown.text, /Zerct/i);

const plaintext = await fetchText("/", { Accept: "text/plain" });
assertIncludes("Plain text content negotiation", plaintext.text, /Zerct/i);

const mcpDiscovery = await fetchText("/.well-known/mcp");
assertIncludes("MCP discovery", mcpDiscovery.text, /"url"\s*:/);
assertIncludes("MCP discovery", mcpDiscovery.text, /\/mcp/);

console.log(`Mintlify agent readiness checks passed for ${baseUrl}`);
