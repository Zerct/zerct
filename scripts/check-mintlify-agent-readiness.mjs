const target = process.argv[2] ?? "https://docs.tovuk.com";
const normalizedTarget = target.startsWith("http") ? target : `https://${target}`;
const baseUrl = normalizedTarget.replace(/\/$/, "");
const fetchRetries = Number.parseInt(process.env.TOVUK_DOCS_CHECK_RETRIES ?? "8", 10);
const fetchRetryDelayMs = Number.parseInt(process.env.TOVUK_DOCS_CHECK_RETRY_DELAY_MS ?? "5000", 10);

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

const retryableErrorCodes = new Set([
  "EAI_AGAIN",
  "ECONNRESET",
  "ENOTFOUND",
  "ETIMEDOUT",
  "UND_ERR_CONNECT_TIMEOUT"
]);

function sleep(ms) {
  return new Promise((resolve) => {
    setTimeout(resolve, ms);
  });
}

function isRetryableFetchError(error) {
  const code = error?.cause?.code ?? error?.code;
  return retryableErrorCodes.has(code);
}

async function requestText(path, headers = {}) {
  const response = await fetch(`${baseUrl}${path}`, { headers });
  const text = await response.text();

  if (!response.ok) {
    const error = new Error(`${path} returned ${response.status}`);
    error.status = response.status;
    throw error;
  }

  return { response, text };
}

async function fetchText(path, headers = {}) {
  let lastError;

  for (let attempt = 0; attempt <= fetchRetries; attempt += 1) {
    try {
      return await requestText(path, headers);
    } catch (error) {
      lastError = error;
      const retryableStatus = error.status === 429 || error.status >= 500;
      const shouldRetry = attempt < fetchRetries && (retryableStatus || isRetryableFetchError(error));

      if (!shouldRetry) {
        throw error;
      }

      await sleep(fetchRetryDelayMs);
    }
  }

  throw lastError;
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
assertIncludes("skill.md", skill.text, /name:\s*tovuk/i);

const robots = await fetchText("/robots.txt");
if (/Disallow:\s*\//i.test(robots.text) && !/Allow:\s*\//i.test(robots.text)) {
  throw new Error("robots.txt appears to block crawlers");
}

const markdown = await fetchText("/", { Accept: "text/markdown" });
assertIncludes("Markdown content negotiation", markdown.text, /Tovuk/i);

const plaintext = await fetchText("/", { Accept: "text/plain" });
assertIncludes("Plain text content negotiation", plaintext.text, /Tovuk/i);

const mcpDiscovery = await fetchText("/.well-known/mcp");
assertIncludes("MCP discovery", mcpDiscovery.text, /"url"\s*:/);
assertIncludes("MCP discovery", mcpDiscovery.text, /\/mcp/);

console.log(`Mintlify agent readiness checks passed for ${baseUrl}`);
