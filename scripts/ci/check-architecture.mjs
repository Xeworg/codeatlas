#!/usr/bin/env node
// scripts/ci/check-architecture.mjs
//
// Architecture guard for the codeatlas hexagonal boundary.
// STRICT POLICY: This script does NOT honor any comment-based opt-out
// (e.g. `// arch-allow: ...`). If a forbidden import is reintroduced,
// the build fails. Exceptions require a separate PR with the
// `arch-exception` label and a follow-up spec change.
//
// Usage:
//   node scripts/ci/check-architecture.mjs                  # default scan
//   node scripts/ci/check-architecture.mjs --self-test      # run self-test against fixtures
//
// Exit codes:
//   0 — no violations found
//   1 — at least one forbidden pattern detected (or self-test failed)

import { readFileSync, existsSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const REPO_ROOT = resolve(__dirname, "..", "..");

// Files scanned in default mode.
const SCAN_TARGETS = [
  resolve(REPO_ROOT, "src-tauri/src/commands.rs"),
];

// Forbidden Rust import patterns. Each entry: { id, regex, description }.
// `regex` is matched against each line of the scanned file. Multi-line
// imports are not supported (Rust single-line `use` statements only).
const FORBIDDEN_PATTERNS = [
  {
    id: "ENGINE_DB_CONCRETE",
    regex: /^\s*use\s+engine::db::/,
    description:
      "Do not import concrete types from engine::db into the presentation layer. Use the port traits in engine::ports (e.g. ScanRepository, GraphRepository, WorkspaceRepository, AnalysisRepository).",
  },
  {
    id: "ENGINE_AI_ANTHROPIC",
    regex: /^\s*use\s+engine::ai::anthropic\b/,
    description:
      "Do not import the private AnthropicProvider adapter. The AI provider must be reached via the public AIService / AIServicePort surface.",
  },
  {
    id: "ENGINE_AI_RESOLVED",
    regex: /^\s*use\s+engine::ai::resolved\b/,
    description:
      "Do not import the private ResolvedProvider enum. Provider resolution is internal to engine::ai::factory.",
  },
  {
    id: "ENGINE_AI_PROVIDER",
    regex: /^\s*use\s+engine::ai::provider::AIProvider\b/,
    description:
      "Do not import the private AIProvider trait. Use AIServicePort (added in PR-B of pre-wave-2-foundation).",
  },
  {
    id: "ENGINE_AI_AISERVICE_CONCRETE",
    regex: /^\s*use\s+engine::ai::AIService\b/,
    description:
      "Do not import the concrete AIService struct. Use Arc<dyn AIServicePort> (PR-B) or the existing engine::ai module re-exports.",
  },
];

function scanFile(filePath) {
  const violations = [];
  if (!existsSync(filePath)) {
    return violations;
  }
  const content = readFileSync(filePath, "utf8");
  const lines = content.split("\n");
  lines.forEach((line, index) => {
    for (const pattern of FORBIDDEN_PATTERNS) {
      if (pattern.regex.test(line)) {
        violations.push({
          file: filePath,
          line: index + 1,
          patternId: pattern.id,
          snippet: line.trim(),
          description: pattern.description,
        });
      }
    }
  });
  return violations;
}

function runDefaultScan() {
  const allViolations = [];
  for (const target of SCAN_TARGETS) {
    allViolations.push(...scanFile(target));
  }
  if (allViolations.length === 0) {
    console.log("Architecture guard: no violations found.");
    process.exit(0);
  }
  console.error("Architecture guard: forbidden imports detected.");
  console.error("");
  for (const v of allViolations) {
    const relPath = v.file.replace(REPO_ROOT + "/", "");
    console.error(`  ${relPath}:${v.line}  [${v.patternId}]`);
    console.error(`    > ${v.snippet}`);
    console.error(`    ${v.description}`);
    console.error("");
  }
  console.error(`Total: ${allViolations.length} violation(s).`);
  process.exit(1);
}

function runSelfTest() {
  const forbiddenFixture = resolve(
    __dirname,
    "__fixtures__/forbidden.rs",
  );
  const cleanFixture = resolve(__dirname, "__fixtures__/clean.rs");

  const forbiddenHits = scanFile(forbiddenFixture);
  const cleanHits = scanFile(cleanFixture);

  let failed = false;
  if (forbiddenHits.length === 0) {
    console.error(
      "Self-test FAILED: expected forbidden fixture to produce violations, got 0.",
    );
    failed = true;
  } else {
    console.log(
      `Self-test OK: forbidden fixture produced ${forbiddenHits.length} violations.`,
    );
  }
  if (cleanHits.length !== 0) {
    console.error(
      `Self-test FAILED: expected clean fixture to produce 0 violations, got ${cleanHits.length}.`,
    );
    failed = true;
  } else {
    console.log("Self-test OK: clean fixture produced 0 violations.");
  }
  process.exit(failed ? 1 : 0);
}

const args = process.argv.slice(2);
if (args.includes("--self-test")) {
  runSelfTest();
} else {
  runDefaultScan();
}