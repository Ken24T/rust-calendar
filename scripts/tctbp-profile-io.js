#!/usr/bin/env node

const fs = require("fs");
const path = require("path");
const { resolvePolicyPath, resolveRepoRoot } = require("./tctbp-runtime");

const repoRoot = resolveRepoRoot();
const policyPath = resolvePolicyPath(repoRoot);

function fail(message) {
  console.error(message);
  process.exit(1);
}

// ── Policy loading ──────────────────────────────────────────────────────────

function loadPolicy() {
  try {
    return JSON.parse(fs.readFileSync(policyPath, "utf8"));
  } catch (error) {
    fail(`Could not read ${policyPath}: ${error instanceof Error ? error.message : String(error)}`);
  }
}

// ── Path resolution ─────────────────────────────────────────────────────────

function resolveRepoPath(relativePath) {
  return path.join(repoRoot, relativePath);
}

// ── JSON file I/O ───────────────────────────────────────────────────────────

function readJsonFile(filePath) {
  try {
    return JSON.parse(fs.readFileSync(filePath, "utf8"));
  } catch (error) {
    fail(`Could not read ${filePath}: ${error instanceof Error ? error.message : String(error)}`);
  }
}

function maybeReadJsonFile(filePath) {
  try {
    return JSON.parse(fs.readFileSync(filePath, "utf8"));
  } catch (_error) {
    return null;
  }
}

function updateJsonFileRaw(filePath, replacements) {
  let content = fs.readFileSync(filePath, "utf8");

  for (const [needle, replacementValue] of Object.entries(replacements)) {
    const pattern = new RegExp(escapeRegExp(needle), "g");
    content = content.replace(pattern, replacementValue);
  }

  fs.writeFileSync(filePath, content, "utf8");
}

// ── Version reading & writing (multi-format: JSON / TOML / plain text) ──────

/**
 * Detects the version-file format from raw content. Version files are
 * stack-agnostic: JSON (package.json), TOML (Cargo.toml), or a short
 * plain-text file (VERSION). Anything else is unsupported.
 */
function detectVersionFileFormat(content) {
  if (!content) return null;
  const trimmed = content.trim();
  if (trimmed.startsWith("{")) return "json";
  if (/^\s*\[package\]\s*$/m.test(content)) return "toml";
  if (trimmed.length > 0 && trimmed.length <= 64) return "plain";
  return null;
}

/** Reads the `version = "x.y.z"` value from the `[package]` table of a TOML file. */
function parseTomlPackageVersion(content) {
  const packageMatch = content.match(/^\s*\[package\]\s*$/m);
  if (!packageMatch) return null;
  const afterPackage = content.slice(packageMatch.index + packageMatch[0].length);
  const nextTable = afterPackage.match(/^\s*\[[^\]]+\]\s*$/m);
  const section = nextTable ? afterPackage.slice(0, nextTable.index) : afterPackage;
  const versionLine = section.match(/^\s*version\s*=\s*"([^"]+)"\s*$/m);
  return versionLine ? versionLine[1] : null;
}

/** Returns the TOML content with the `[package]` version replaced, or null if not found. */
function renderTomlPackageVersion(content, version) {
  const packageMatch = content.match(/^\s*\[package\]\s*$/m);
  if (!packageMatch) return null;
  const afterPackage = content.slice(packageMatch.index + packageMatch[0].length);
  const nextTable = afterPackage.match(/^\s*\[[^\]]+\]\s*$/m);
  const section = nextTable ? afterPackage.slice(0, nextTable.index) : afterPackage;
  const versionLine = section.match(/^(\s*version\s*=\s*)"[^"]*"(\r?\n?)$/m);
  if (!versionLine) return null;
  const absoluteStart = packageMatch.index + packageMatch[0].length + versionLine.index;
  const updatedLine = `${versionLine[1]}"${version}"${versionLine[2] || ""}`;
  return content.slice(0, absoluteStart) + updatedLine + content.slice(absoluteStart + versionLine[0].length);
}

/** Reads the version from a file, detecting its format. Never exits the process. */
function readVersionFile(filePath) {
  let content;
  try {
    content = fs.readFileSync(filePath, "utf8");
  } catch (error) {
    return { ok: false, error: `Could not read ${filePath}: ${error instanceof Error ? error.message : String(error)}` };
  }
  const format = detectVersionFileFormat(content);
  if (format === "json") {
    try {
      const json = JSON.parse(content);
      if (typeof json.version === "string") return { ok: true, format, version: json.version };
      return { ok: false, error: `No string 'version' field in ${filePath}.` };
    } catch (error) {
      return { ok: false, error: `Could not parse ${filePath} as JSON: ${error instanceof Error ? error.message : String(error)}` };
    }
  }
  if (format === "toml") {
    const version = parseTomlPackageVersion(content);
    return version === null
      ? { ok: false, error: `No 'version' field under [package] in ${filePath}.` }
      : { ok: true, format, version };
  }
  if (format === "plain") {
    return { ok: true, format, version: content.trim().split(/\r?\n/)[0].trim() };
  }
  return { ok: false, error: `Unsupported version file format: ${filePath}.` };
}

/**
 * Writes a new version into a version file, preserving its format and as much
 * of the original formatting as possible. Returns { ok, format } or { ok:false, error }.
 */
function writeVersionFile(filePath, version, oldVersion) {
  const current = readVersionFile(filePath);
  if (!current.ok) return current;
  if (current.format === "json") {
    if (typeof oldVersion === "string") {
      // Raw string replacement preserves the file's existing formatting.
      updateJsonFileRaw(filePath, { [`"version": "${oldVersion}"`]: `"version": "${version}"` });
    } else {
      const parsed = JSON.parse(fs.readFileSync(filePath, "utf8"));
      parsed.version = version;
      fs.writeFileSync(filePath, `${JSON.stringify(parsed, null, 2)}\n`, "utf8");
    }
    return { ok: true, format: "json" };
  }
  if (current.format === "toml") {
    const rendered = renderTomlPackageVersion(fs.readFileSync(filePath, "utf8"), version);
    if (rendered === null) return { ok: false, error: `No 'version' field under [package] in ${filePath}.` };
    fs.writeFileSync(filePath, rendered, "utf8");
    return { ok: true, format: "toml" };
  }
  if (current.format === "plain") {
    fs.writeFileSync(filePath, `${version}\n`, "utf8");
    return { ok: true, format: "plain" };
  }
  return { ok: false, error: `Unsupported version file format: ${filePath}.` };
}

function readVersionSource(config) {
  const relativePath =
    config && config.profile && config.profile.versioning && typeof config.profile.versioning.sourceOfTruth === "string"
      ? config.profile.versioning.sourceOfTruth
      : null;

  if (!relativePath) {
    return {
      path: "n/a",
      version: "n/a"
    };
  }

  const result = readVersionFile(resolveRepoPath(relativePath));
  if (!result.ok) {
    return {
      path: relativePath,
      version: "unknown",
      error: result.error
    };
  }
  return {
    path: relativePath,
    version: result.version
  };
}

// ── Semver ──────────────────────────────────────────────────────────────────

function parseSemVer(version) {
  const match = String(version).match(/^(\d+)\.(\d+)\.(\d+)$/);

  if (!match) {
    fail(`Unsupported version format '${version}' (expected X.Y.Z).`);
  }

  return {
    major: Number.parseInt(match[1], 10),
    minor: Number.parseInt(match[2], 10),
    patch: Number.parseInt(match[3], 10)
  };
}

function stepSemVer(version, bump) {
  const parsed = parseSemVer(version);

  if (bump === "minor") {
    return `${parsed.major}.${parsed.minor + 1}.0`;
  }

  if (bump === "major") {
    return `${parsed.major + 1}.0.0`;
  }

  return `${parsed.major}.${parsed.minor}.${parsed.patch + 1}`;
}

// ── Release tag resolution ──────────────────────────────────────────────────

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function getReleaseTagPattern(config) {
  const configuredFormat =
    config && config.profile && config.profile.versioning && typeof config.profile.versioning.tagFormat === "string"
      ? config.profile.versioning.tagFormat
      : "v{version}";
  const patternSource = configuredFormat
    .split("{version}")
    .map((segment) => escapeRegExp(segment))
    .join("\\d+\\.\\d+\\.\\d+");

  return new RegExp(`^${patternSource}$`);
}

function getReleaseTagGlob(config) {
  const configuredFormat =
    config && config.profile && config.profile.versioning && typeof config.profile.versioning.tagFormat === "string"
      ? config.profile.versioning.tagFormat
      : "v{version}";

  return configuredFormat.replace("{version}", "*");
}

// ── Shared target resolution ────────────────────────────────────────────────

function resolveTarget(targets, targetArg) {
  const normalized = String(targetArg).toLowerCase();

  for (const [key, target] of Object.entries(targets)) {
    const names = [key, ...(target.aliases || [])].map((value) => String(value).toLowerCase());

    if (names.includes(normalized)) {
      return { key, target };
    }
  }

  return null;
}

module.exports = {
  detectVersionFileFormat,
  getReleaseTagGlob,
  getReleaseTagPattern,
  loadPolicy,
  maybeReadJsonFile,
  parseSemVer,
  parseTomlPackageVersion,
  policyPath,
  readJsonFile,
  readVersionFile,
  readVersionSource,
  renderTomlPackageVersion,
  repoRoot,
  resolveRepoPath,
  resolveTarget,
  stepSemVer,
  updateJsonFileRaw,
  writeVersionFile
};
