import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";

const proseExtensions = new Set([".md", ".mdx", ".txt"]);
const ignoredPaths = new Set(["docs/robots.txt"]);
const ignoredBinaryExtensions = new Set([
  ".avif",
  ".gif",
  ".ico",
  ".jpeg",
  ".jpg",
  ".otf",
  ".pdf",
  ".png",
  ".ttf",
  ".webp",
  ".woff",
  ".woff2"
]);

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
const files = gitFiles().filter((file) => !ignoredPaths.has(file) && existsSync(file));
const textFiles = [];
const emDash = String.fromCodePoint(0x2014);

for (const file of files) {
  if (ignoredBinaryExtensions.has(path.extname(file))) {
    continue;
  }

  const contents = readFileSync(file);
  if (contents.includes(0)) {
    continue;
  }

  const text = contents.toString("utf8");
  textFiles.push({ file, text });

  const lines = text.split(/\r?\n/);
  lines.forEach((line, lineIndex) => {
    for (const column of findAll(line, emDash)) {
      findings.push({
        column,
        file,
        line: lineIndex + 1,
        message: "em dash is not allowed in any tracked text file"
      });
    }
  });
}

const proseFiles = textFiles.filter(({ file }) => proseExtensions.has(path.extname(file)));

for (const { file, text } of proseFiles) {
  const lines = text.split(/\r?\n/);
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
  console.error("Style check failed.");
  console.error("Do not use em dashes in tracked text files.");
  console.error("Do not use double hyphen prose outside inline code or fenced code blocks.");

  for (const finding of findings) {
    console.error(`${finding.file}:${finding.line}:${finding.column}: ${finding.message}`);
  }

  process.exit(1);
}

console.log(`Checked ${textFiles.length} text files for em dashes.`);
console.log(`Checked ${proseFiles.length} prose files for double hyphen prose.`);
