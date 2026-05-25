const projectId = process.env.MINTLIFY_PROJECT_ID;
const adminApiKey = process.env.MINTLIFY_ADMIN_API_KEY;
const pollIntervalMs = 5_000;
const maxPolls = 120;

if (!projectId) {
  throw new Error("MINTLIFY_PROJECT_ID is required");
}

if (!adminApiKey) {
  throw new Error("MINTLIFY_ADMIN_API_KEY is required");
}

async function mintlifyFetch(path, options = {}) {
  const response = await fetch(`https://api.mintlify.com/v1${path}`, {
    ...options,
    headers: {
      ...options.headers,
      Authorization: `Bearer ${adminApiKey}`
    }
  });
  const text = await response.text();
  const body = text ? JSON.parse(text) : {};

  if (!response.ok) {
    throw new Error(`Mintlify API returned ${response.status}: ${body.message ?? text}`);
  }

  return body;
}

function isDomainRevalidationOnlyFailure(status) {
  const deploymentText = [status.summary, ...(status.logs ?? [])]
    .filter(Boolean)
    .join("\n");

  return (
    deploymentText.includes("Failed to revalidate domain: docs.zerct.com") &&
    deploymentText.includes("Successfully updated deployment") &&
    deploymentText.includes("Successfully indexed")
  );
}

const update = await mintlifyFetch(`/project/update/${projectId}`, {
  method: "POST"
});

if (!update.statusId) {
  throw new Error("Mintlify deployment response did not include statusId");
}

console.log(`Mintlify deployment queued: ${update.statusId}`);

for (let attempt = 1; attempt <= maxPolls; attempt += 1) {
  const status = await mintlifyFetch(`/project/update-status/${update.statusId}`);
  console.log(`Mintlify deployment status: ${status.status}`);

  if (status.status === "success") {
    console.log(status.summary ?? "Mintlify deployment succeeded");
    process.exit(0);
  }

  if (status.status === "failure") {
    if (isDomainRevalidationOnlyFailure(status)) {
      console.warn("Mintlify deployment updated; docs.zerct.com revalidation is pending DNS cutover.");
      console.warn(status.summary);
      process.exit(0);
    }

    console.error(status.summary ?? "Mintlify deployment failed");
    for (const line of status.logs ?? []) {
      console.error(line);
    }
    process.exit(1);
  }

  await new Promise((resolve) => {
    setTimeout(resolve, pollIntervalMs);
  });
}

throw new Error("Timed out waiting for Mintlify deployment");
