import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import path from "node:path";

const proseExtensions = new Set([".md", ".mdx", ".txt"]);
const ignoredPaths = new Set(["docs/robots.txt"]);

function gitFiles() {
  const output = execFileSync(
    "git",
    ["ls-files", "--cached", "--others", "--exclude-standard"],
    { encoding: "utf8" }
  );

  return output
    .split("\n")
    .map((file) => file.trim())
    .filter(Boolean);
}

function blankRange(line, start, end) {
  return `${line.slice(0, start)}${" ".repeat(end - start)}${line.slice(end)}`;
}

function stripInlineCode(line) {
  let stripped = line;
  let index = 0;

  while (index < stripped.length) {
    if (stripped[index] !== "`") {
      index += 1;
      continue;
    }

    let tickCount = 1;
    while (stripped[index + tickCount] === "`") {
      tickCount += 1;
    }

    const fence = "`".repeat(tickCount);
    const end = stripped.indexOf(fence, index + tickCount);
    if (end === -1) {
      index += tickCount;
      continue;
    }

    stripped = blankRange(stripped, index, end + tickCount);
    index = end + tickCount;
  }

  return stripped;
}

function stripUrls(line) {
  return line.replace(/https?:\/\/\S+/g, (match) => " ".repeat(match.length));
}

function stripInlineHtmlComments(line) {
  return line.replace(/<!--.*?-->/g, (match) => " ".repeat(match.length));
}

function searchableLine(line) {
  return stripInlineHtmlComments(stripUrls(stripInlineCode(line)));
}

function findAll(line, needle) {
  const columns = [];
  let index = line.indexOf(needle);

  while (index !== -1) {
    columns.push(index + 1);
    index = line.indexOf(needle, index + needle.length);
  }

  return columns;
}

const findings = [];
const files = gitFiles().filter((file) => {
  return proseExtensions.has(path.extname(file)) && !ignoredPaths.has(file);
});

for (const file of files) {
  const lines = readFileSync(file, "utf8").split(/\r?\n/);
  let inFence = false;
  let fenceMarker = "";

  lines.forEach((line, lineIndex) => {
    const trimmed = line.trim();
    const fenceMatch = line.match(/^\s*(`{3,}|~{3,})/);

    if (fenceMatch && (!inFence || fenceMatch[1].startsWith(fenceMarker[0]))) {
      inFence = !inFence;
      fenceMarker = inFence ? fenceMatch[1] : "";
      return;
    }

    if (inFence || trimmed === "---") {
      return;
    }

    const searchable = searchableLine(line);

    for (const column of findAll(searchable, "—")) {
      findings.push({
        column,
        file,
        line: lineIndex + 1,
        message: "em dash is not allowed in prose"
      });
    }

    for (const column of findAll(searchable, "--")) {
      findings.push({
        column,
        file,
        line: lineIndex + 1,
        message: "double hyphen is not allowed in prose; wrap CLI flags in code"
      });
    }
  });
}

if (findings.length > 0) {
  console.error("Prose style check failed.");
  console.error("Do not use em dashes or double hyphen prose outside inline code or fenced code blocks.");

  for (const finding of findings) {
    console.error(`${finding.file}:${finding.line}:${finding.column}: ${finding.message}`);
  }

  process.exit(1);
}

console.log(`Checked ${files.length} prose files for em dashes and double hyphen prose.`);
